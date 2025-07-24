use std::sync::Arc;
use tokio::sync::Mutex;
use axum::{
    Router,
    extract::State,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::any,
};
use wars::protocol::{self, ActionMessage, EventMessage};

use std::{collections::{HashMap, HashSet}, net::SocketAddr, path::PathBuf};
use tower_http::{
    services::ServeDir,
    trace::{DefaultMakeSpan, TraceLayer},
};

use futures_util::{Sink, Stream, SinkExt,StreamExt, stream::{SplitStream, SplitSink}};
use axum::extract::connect_info::ConnectInfo;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod model;
mod state;

type SenderId = usize;
type SubscriptionId = usize;
type SenderSocket = SplitSink<WebSocket, Message>;
type SenderMessage = Message;

struct Sender {
    senders: HashMap<SenderId, SenderSocket>,
    next_sender_id: SenderId,
    subscriptions: HashMap<SubscriptionId, HashSet<SenderId>>
}

impl Sender {
    fn new() -> Self {
        Self {
            senders: HashMap::new(),
            next_sender_id: 1,
            subscriptions: HashMap::new()
        }
    }
    fn add_sender(&mut self, sender: SenderSocket) -> SenderId {
        let sender_id = self.next_sender_id;
        self.next_sender_id += 1;
        self.senders.insert(sender_id, sender);
        sender_id
    }
    fn subscribe(&mut self, sender_id: SenderId, subscription_id: SubscriptionId) {
        let subscription = self.subscriptions.entry(subscription_id).or_default();
        subscription.insert(sender_id);
    }
    async fn send(&mut self, sender_id: &SenderId, message: Message) -> Result<(), axum::Error> {
        let Some(sender) = self.senders.get_mut(sender_id) else {
            return Ok(())
        };
        sender.send(message).await
    }
    async fn send_subscribers(&mut self, subscription_id: &SubscriptionId, message: Message) -> Result<(), axum::Error> {
        let Some(sender_ids) = self.subscriptions.get(subscription_id) else {
            return Ok(())
        };
        let mut result = Ok(());
        for sender_id in sender_ids {
            let Some(sender) = self.senders.get_mut(sender_id) else {
                return Ok(())
            };
            
            if let Err(e) = sender.send(message.clone()).await{
                result = Err(e);
            }
        }
        result
    }
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");

    let database_pool = model::new_database_pool("wars.db").await?;
    let sender = Arc::new(Mutex::new(Sender::new()));

    let app = Router::new()
        .fallback_service(ServeDir::new(assets_dir).append_index_html_on_directories(true))
        .route("/ws", any(ws_handler))
        .with_state((database_pool, sender))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State((pool, sender)): State<(model::DatabasePool, Arc<Mutex<Sender>>)>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let sender = Arc::clone(&sender);
    ws.on_upgrade(async move |socket| {
        let (write, read) = socket.split();
        let sender_id = sender.lock().await.add_sender(write);
        let _ = handle_socket(read, sender, sender_id,addr, pool).await;
    })
}

async fn handle_socket(
    mut read: SplitStream<WebSocket>,
    sender: Arc<Mutex<Sender>>,
    sender_id: SenderId,
    who: SocketAddr,
    pool: model::DatabasePool,
) -> Result<(), axum::Error> {
    let mut state = state::State {};

    sender.lock().await.send(&sender_id, serialize_event(&EventMessage::ServerVersion(protocol::VERSION.to_owned()), false)).await?;

    while let Some(Ok(msg)) = read.next().await {
        let binary = matches!(msg, Message::Binary(_));

        let action = parse_action(msg, who);

        // Protocol level processing
        match action {
            Ok(ActionMessage::Quit) => break,
            Ok(ActionMessage::SubscribeGame(game_id)) => sender.lock().await.subscribe(sender_id, game_id as usize),
            Err(_) => break,
            _ => ()
        };

        // State level processing
        if let Ok(action) = action {
            let events = state.action(action, &pool).await;

            for (recipient, event) in events {
                match recipient {
                    state::Recipient::Actor => {
                        sender.lock().await.send(&sender_id, serialize_event(&event, binary)).await?
                    }
                    state::Recipient::Subscribers(game_id) => {
                        sender.lock().await.send_subscribers(&(game_id as usize), serialize_event(&event, binary)).await?
                    }
                }
            }
        }
    }

    Ok(())
}

fn parse_action(msg: Message, _who: SocketAddr) -> Result<ActionMessage, protocol::Error> {
    match msg {
        Message::Text(t) => ActionMessage::from_text(&t),
        Message::Binary(d) => ActionMessage::from_bytes(&d),
        Message::Close(_) => Ok(ActionMessage::Quit),
        _ => Ok(ActionMessage::NoOp)
    }
}

fn serialize_event(event: &EventMessage, binary: bool) -> Message {
    if binary {
        Message::Binary(event.as_bytes().unwrap().into())
    } else {
        Message::Text(event.as_text().unwrap().into())
    }
}

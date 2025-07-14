use axum::{
    Router,
    body::Bytes,
    extract::State,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::any,
};

use std::{net::SocketAddr, ops::ControlFlow, path::PathBuf};
use tower_http::{
    services::ServeDir,
    trace::{DefaultMakeSpan, TraceLayer},
};

use axum::extract::connect_info::ConnectInfo;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod model;
mod state;

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

    let database_pool = model::new_database_pool("sqlite:wars.db").await?;

    let app = Router::new()
        .fallback_service(ServeDir::new(assets_dir).append_index_html_on_directories(true))
        .route("/ws", any(ws_handler))
        .with_state(database_pool)
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
    State(pool): State<model::DatabasePool>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    ws.on_upgrade(async move |socket| {
        let _ = handle_socket(socket, addr, pool).await;
    })
}

async fn handle_socket(
    mut socket: WebSocket,
    who: SocketAddr,
    pool: model::DatabasePool,
) -> Result<(), axum::Error> {
    let mut state = state::State {};

    while let Some(Ok(msg)) = socket.recv().await {
        let binary = matches!(msg, Message::Binary(_));

        match parse_action(msg, who) {
            ControlFlow::Continue(action) => {
                let events = state.action(action, &pool).await;

                for (recipient, event) in events {
                    match recipient {
                        state::Recipient::Actor => {
                            socket.send(serialize_event(&event, binary)).await?
                        }
                        state::Recipient::Subscribers => {
                            // TODO: Send all subscribers
                            socket.send(serialize_event(&event, binary)).await?
                        }
                    }
                }
            }
            ControlFlow::Break(_) => break,
        }
    }

    Ok(())
}

fn parse_action(msg: Message, _who: SocketAddr) -> ControlFlow<(), state::Action> {
    match msg {
        Message::Text(t) => match serde_json::from_str(t.as_str()) {
            Ok(action) => ControlFlow::Continue(action),
            Err(_) => ControlFlow::Break(()),
        },
        Message::Binary(d) => match postcard::from_bytes(&d) {
            Ok(action) => ControlFlow::Continue(action),
            Err(_) => ControlFlow::Break(()),
        },
        Message::Close(_c) => ControlFlow::Break(()),
        _ => ControlFlow::Continue(state::Action::NoOp),
    }
    .into()
}

fn serialize_event(event: &state::Event, binary: bool) -> Message {
    if binary {
        Message::Binary(Bytes::from(postcard::to_allocvec(event).unwrap()))
    } else {
        Message::Text(serde_json::to_string(event).unwrap().into())
    }
}

use sqlx::prelude::*;

pub type DatabasePool = sqlx::Pool<sqlx::Sqlite>;
pub type DatabaseError = sqlx::Error;
pub type DatabaseResult<T> = Result<T, DatabaseError>;

pub type GameId = u32;
pub type EventIndex = u32;
#[derive(FromRow)]
pub struct Game {
    pub id: GameId,
    pub last_event_index: EventIndex,
    pub data: String,
}

#[derive(FromRow)]
pub struct GameEvent {
    pub game_id: GameId,
    pub index: EventIndex,
    pub data: String
}

pub async fn new_database_pool(connection_string: &str) -> anyhow::Result<DatabasePool> {
    let sqlite_opts = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(connection_string)
        .create_if_missing(true);
    Ok(sqlx::sqlite::SqlitePool::connect_with(sqlite_opts).await?)
}

pub async fn load_game(game_id: GameId, pool: &DatabasePool) -> DatabaseResult<(wars::game::Game, EventIndex)> {
    sqlx::query_as("select * from games where id = (?1)")
        .bind(game_id)
        .fetch_one(pool)
        .await
        .map(|game: Game| (ron::from_str(&game.data).unwrap(), game.last_event_index))
}
pub async fn save_game(
    game_id: GameId,
    game: wars::game::Game,
    new_events: impl IntoIterator<Item=wars::game::Event>,
    pool: &DatabasePool,
) -> DatabaseResult<EventIndex> {
    let _transaction = pool.begin().await?;

    let mut last_event_index: EventIndex = sqlx::query_scalar("select last_event_index from games where id = ?1").bind(game_id).fetch_one(pool).await?;
    for event in new_events.into_iter() {
        let data = ron::to_string(&event).unwrap();

        last_event_index += 1;
        sqlx::query("insert into game_events(game_id, index, data) values (?1, ?2, ?3)")
            .bind(game_id)
            .bind(last_event_index)
            .bind(data)
            .execute(pool)
            .await?;
    }
    
    let data = ron::to_string(&game).unwrap();

    sqlx::query("update games set data = ?1, last_event_index = ?2 where id = (?3))")
        .bind(game_id)
        .bind(data)
        .bind(last_event_index)
        .execute(pool)
        .await?;
    Ok(last_event_index)
}
pub async fn create_game(game: wars::game::Game, pool: &DatabasePool) -> DatabaseResult<GameId> {
    let data = ron::to_string(&game).unwrap();

    sqlx::query_scalar("insert into games(data, last_event_index) values (?1, 0) returning id")
        .bind(data)
        .fetch_one(pool)
        .await
}
pub async fn load_game_events(game_id: GameId, since: Option<EventIndex>, pool: &DatabasePool) -> DatabaseResult<Vec<(EventIndex, wars::game::Event)>> {
    let result = if let Some(since) = since {
        sqlx::query_as("select * from game_events where game_id = ?1 and index > ?2")
            .bind(game_id)
            .bind(since)
            .fetch_all(pool).await?
            .into_iter()
            .map(|e: GameEvent| (e.index, ron::from_str::<wars::game::Event>(&e.data).unwrap()))
            .collect::<Vec<(EventIndex, wars::game::Event)>>()
    } else {
        sqlx::query_as("select * from game_events where game_id = ?1")
            .bind(game_id)
            .fetch_all(pool).await?
            .into_iter()
            .map(|e: GameEvent| (e.index, ron::from_str::<wars::game::Event>(&e.data).unwrap()))
            .collect()
    };
    Ok(result)
}

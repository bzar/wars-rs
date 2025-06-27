use sqlx::prelude::*;

pub type DatabasePool = sqlx::Pool<sqlx::Sqlite>;
pub type DatabaseError = sqlx::Error;
pub type DatabaseResult<T> = Result<T, DatabaseError>;

pub type GameId = u32;

#[derive(FromRow)]
pub struct Game {
    id: GameId,
    data: String,
}

pub async fn new_database_pool(connection_string: &str) -> anyhow::Result<DatabasePool> {
    let sqlite_opts = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(connection_string)
        .create_if_missing(true);
    Ok(sqlx::sqlite::SqlitePool::connect_with(sqlite_opts).await?)
}

pub async fn load_game(game_id: GameId, pool: &DatabasePool) -> DatabaseResult<wars::game::Game> {
    sqlx::query_scalar("select data from games where id = (?1)")
        .bind(game_id)
        .fetch_one(pool)
        .await
        .map(|data: String| ron::from_str(&data).unwrap())
}
pub async fn save_game(
    game_id: GameId,
    game: wars::game::Game,
    pool: &DatabasePool,
) -> DatabaseResult<()> {
    let data = ron::to_string(&game).unwrap();

    sqlx::query("update games set data = (?1) where id = (?2))")
        .bind(game_id)
        .bind(data)
        .execute(pool)
        .await?;
    Ok(())
}
pub async fn create_game(game: wars::game::Game, pool: &DatabasePool) -> DatabaseResult<GameId> {
    let data = postcard::to_allocvec(&game).unwrap();

    sqlx::query_scalar("insert into games(data) values (?1) returning id")
        .bind(data)
        .fetch_one(pool)
        .await
}

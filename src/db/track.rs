use chrono::{DateTime, Utc};
use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub(crate) struct Track {
    #[allow(dead_code)]
    pub(crate) id: i64,
    pub(crate) url: String,
    pub(crate) title: String,
    pub(crate) channel: String,
    pub(crate) duration: String,
    pub(crate) thumbnail: String,
    pub(crate) updated: DateTime<Utc>,
}
impl Track {
    pub(crate) fn new(
        url: String,
        title: String,
        channel: String,
        duration: String,
        thumbnail: String,
    ) -> Self {
        Self {
            id: 0,
            url,
            title,
            channel,
            duration,
            thumbnail,
            updated: chrono::offset::Utc::now(),
        }
    }
}

pub(crate) async fn insert_track(
    pool: &sqlx::SqlitePool,
    track: Track,
) -> Result<i64, sqlx::Error> {
    let query = r#"
        INSERT INTO tracks (url, title, channel, duration, thumbnail, updated)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (url) DO UPDATE SET
            title = excluded.title,
            channel = excluded.channel,
            duration = excluded.duration,
            thumbnail = excluded.thumbnail,
            updated = excluded.updated
        "#;

    let res = sqlx::query(query)
        .bind(&track.url)
        .bind(track.title)
        .bind(track.channel)
        .bind(track.duration)
        .bind(track.thumbnail)
        .bind(track.updated)
        .execute(pool)
        .await?;

    let id = res.last_insert_rowid();

    // the track id is 0 if we only updated a row
    if id == 0 {
        let track: Track = sqlx::query_as(
            r#"
            SELECT id FROM tracks
            WHERE url = $1
            "#,
        )
        .bind(track.url)
        .fetch_one(pool)
        .await?;
        return Ok(track.id);
    }
    Ok(id)
}

#[derive(Debug, FromRow)]
pub(crate) struct User {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) global_name: Option<String>,
    pub(crate) updated: DateTime<Utc>,
}

impl User {
    pub(crate) fn new(id: String, name: String, global_name: Option<String>) -> Self {
        Self {
            id,
            name,
            global_name,
            updated: chrono::offset::Utc::now(),
        }
    }
}

pub(crate) async fn insert_user(pool: &sqlx::SqlitePool, user: User) -> Result<(), sqlx::Error> {
    let query = r#"
        INSERT INTO users (id, name, global_name, updated)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (id) DO UPDATE SET
            name = excluded.name,
            global_name = excluded.global_name,
            updated = excluded.updated
        "#;
    sqlx::query(query)
        .bind(user.id)
        .bind(user.name)
        .bind(user.global_name)
        .bind(user.updated)
        .execute(pool)
        .await?;
    Ok(())
}

#[derive(Debug, FromRow)]
pub(crate) struct Query {
    #[allow(dead_code)]
    pub(crate) id: i64,
    pub(crate) user_id: String,
    pub(crate) guild_id: String,
    pub(crate) track_id: i64,
    pub(crate) updated: DateTime<Utc>,
}

impl Query {
    pub(crate) fn new(user_id: String, guild_id: String, track_id: i64) -> Self {
        Self {
            id: 0,
            user_id,
            guild_id,
            track_id,
            updated: chrono::offset::Utc::now(),
        }
    }
}

pub(crate) async fn insert_query(pool: &sqlx::SqlitePool, q: Query) -> Result<i64, sqlx::Error> {
    let query = r#"
        INSERT INTO queries (user_id, guild_id, track_id, updated)
        VALUES ($1, $2, $3, $4)
        "#;
    let res = sqlx::query(query)
        .bind(q.user_id)
        .bind(q.guild_id)
        .bind(q.track_id)
        .bind(q.updated)
        .execute(pool)
        .await?;
    Ok(res.last_insert_rowid())
}

#[derive(Debug, FromRow)]
pub(crate) struct Guild {
    pub(crate) id: String,
    pub(crate) updated: DateTime<Utc>,
}

impl Guild {
    pub(crate) fn new(id: String) -> Self {
        Self {
            id,
            updated: chrono::offset::Utc::now(),
        }
    }
}

pub(crate) async fn insert_guild(pool: &sqlx::SqlitePool, guild: Guild) -> Result<(), sqlx::Error> {
    let query = r#"
        INSERT INTO guilds (id, updated)
        VALUES ($1, $2)
        ON CONFLICT (id) DO UPDATE SET
            updated = excluded.updated
        "#;
    sqlx::query(query)
        .bind(guild.id)
        .bind(guild.updated)
        .execute(pool)
        .await?;
    Ok(())
}

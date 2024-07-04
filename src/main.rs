mod handler;
use handler::Handler;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
mod colors;
mod commands;
mod db;
mod interaction_commands;
mod metadata;
mod signal;
mod state;
mod utils;

use crate::commands::get_chat_commands;
use dotenv::dotenv;
use futures::StreamExt;
use signal::signal_handler;
use songbird::{shards::TwilightMap, Songbird};
use state::StateRef;
use std::{env, error::Error, str::FromStr, sync::Arc, time::Duration};
use tokio::select;
use tracing::{debug, info};
use twilight_cache_inmemory::InMemoryCache;
use twilight_gateway::{
    stream::{self, ShardEventStream},
    Intents, Shard,
};
use twilight_http::Client as HttpClient;
use twilight_model::id::Id;
use twilight_standby::Standby;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    dotenv().ok();

    println!("Starting up...");

    tracing_subscriber::fmt::init();

    info!("Starting up...");

    let (mut shards, state) = {
        let db = env::var("DATABASE_URL").map_err(|_| "DATABASE_URL is not set")?;
        let options = SqliteConnectOptions::from_str(&db)
            .expect("could not create options")
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;
        sqlx::migrate!().run(&pool).await?;

        let token = env::var("DISCORD_TOKEN").map_err(|_| "DISCORD_TOKEN is not set")?;
        let app_id = env::var("DISCORD_APP_ID")
            .map_err(|_| "DISCORD_APP_ID is not set")?
            .parse()?;

        let http = HttpClient::new(token.clone());
        let user_id = http.current_user().await?.model().await?.id;
        let application_id = Id::new(app_id);
        let interaction_client = http.interaction(application_id);

        interaction_client
            .set_global_commands(&get_chat_commands())
            .await?;
        // let commands = interaction_client.global_commands().await?.models().await?;
        // debug!("Global commands: {:?}", commands);

        let intents = Intents::GUILDS
            | Intents::GUILD_MESSAGES
            | Intents::GUILD_VOICE_STATES
            | Intents::MESSAGE_CONTENT;
        let config = twilight_gateway::Config::new(token.clone(), intents);
        let shards: Vec<Shard> =
            stream::create_recommended(&http, config, |_, builder| builder.build())
                .await?
                .collect();
        let senders = TwilightMap::new(
            shards
                .iter()
                .map(|s| (s.id().number(), s.sender()))
                .collect(),
        );
        let songbird = Songbird::twilight(Arc::new(senders), user_id);
        let cache = InMemoryCache::new();
        let client = reqwest::ClientBuilder::new()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(3600))
            .build()
            .expect("could not build http client");

        (
            shards,
            Arc::new(StateRef {
                http,
                cache,
                songbird,
                standby: Standby::new(),
                guild_settings: Default::default(),
                pool,
                client,
            }),
        )
    };

    info!("Ready to receive events");

    let handler = Handler::new(Arc::clone(&state));
    let mut stop_rx = signal_handler();
    let mut stream = ShardEventStream::new(shards.iter_mut());
    loop {
        select! {
            biased;
            _ = stop_rx.changed() => {
                for guild in state.cache.iter().guilds() {
                    if let Some(user) = state.cache.current_user() {
                        if state.cache.voice_state(user.id, guild.id()).is_some() {
                            debug!("Leaving guild {:?}", guild.id());
                            state.songbird.leave(guild.id()).await?;
                        }
                    }
                }
                // need to grab next event to properly leave voice channels
                stream.next().await;
                break;
            },
            next = stream.next() => {
                let event = match next {
                    Some((_, Ok(event))) => event,
                    Some((_, Err(source))) => {
                        tracing::warn!(?source, "error receiving event");
                        if source.is_fatal() {
                            break;
                        }
                        continue;
                    }
                    None => break,
                };
                debug!("Event: {:?}", &event);

                state.cache.update(&event);
                state.standby.process(&event);
                state.songbird.process(&event).await;

                handler.act(event).await?;
            }
        }
    }
    Ok(())
}

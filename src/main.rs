mod handler;
use handler::Handler;
mod commands;
mod metadata;
mod state;
use dotenv::dotenv;
use futures::StreamExt;
use songbird::{shards::TwilightMap, Songbird};
use state::StateRef;
use std::{env, error::Error, sync::Arc};
use tokio::{
    select,
    signal::unix::{signal, SignalKind},
    sync::watch,
};
use tracing::{debug, info};
use twilight_cache_inmemory::InMemoryCache;
use twilight_gateway::{
    stream::{self, ShardEventStream},
    Intents, Shard,
};
use twilight_http::Client as HttpClient;
use twilight_model::application::command::CommandType;
use twilight_model::id::Id;
use twilight_standby::Standby;
use twilight_util::builder::command::{CommandBuilder, StringBuilder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    dotenv().ok();

    // Initialize the tracing subscriber.
    tracing_subscriber::fmt::init();

    let (stop_tx, mut stop_rx) = watch::channel(());

    tokio::spawn(async move {
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        let mut sigint = signal(SignalKind::interrupt()).unwrap();
        loop {
            select! {
                _ = sigterm.recv() => println!("Receive SIGTERM"),
                _ = sigint.recv() => println!("Receive SIGTERM"),
            };
            stop_tx.send(()).unwrap();
        }
    });

    let (mut shards, state) = {
        let token = env::var("DISCORD_TOKEN")?;
        let app_id = env::var("DISCORD_APP_ID")?.parse()?;

        let http = HttpClient::new(token.clone());
        let user_id = http.current_user().await?.model().await?.id;
        let application_id = Id::new(app_id);
        let interaction_client = http.interaction(application_id);

        let commands = &[
            CommandBuilder::new("play", "Add a song to the queue", CommandType::ChatInput)
                .option(StringBuilder::new("query", "URL of a song").required(true))
                .build(),
            CommandBuilder::new("stop", "Stop playing", CommandType::ChatInput).build(),
            CommandBuilder::new("join", "Join the channel", CommandType::ChatInput).build(),
            CommandBuilder::new("leave", "Leave the channel", CommandType::ChatInput).build(),
        ];
        interaction_client.set_global_commands(commands).await?;

        let commands = interaction_client.global_commands().await?.models().await?;
        debug!("Global commands: {:?}", commands);

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

        (
            shards,
            Arc::new(StateRef {
                http,
                cache,
                songbird,
                standby: Standby::new(),
            }),
        )
    };

    let mut handler = Handler::new(Arc::clone(&state));
    let mut stream = ShardEventStream::new(shards.iter_mut());
    loop {
        select! {
            biased;
            _ = stop_rx.changed() => {
                for guild in state.cache.iter().guilds(){
                    info!("Leaving guild {:?}", guild.id());
                    state.songbird.leave(guild.id()).await?;
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

                handler.act(event).await;
            }
        }
    }
    Ok(())
}

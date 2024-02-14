use dotenv::dotenv;
use futures::StreamExt;
use songbird::{
    input::{Compose, YoutubeDl},
    shards::TwilightMap,
    Songbird,
};
use std::{
    env, error::Error, future::Future, num::NonZeroU64, ops::Sub, sync::Arc, time::Duration,
};
use tracing::debug;
use twilight_cache_inmemory::InMemoryCache;
use twilight_gateway::{
    stream::{self, ShardEventStream},
    Event, Intents, Shard,
};
use twilight_http::Client as HttpClient;
use twilight_model::application::command::CommandType;
use twilight_model::{channel::Message, id::Id};
use twilight_standby::Standby;
use twilight_util::builder::command::{CommandBuilder, StringBuilder};

type State = Arc<StateRef>;

#[derive(Debug)]
struct StateRef {
    http: HttpClient,
    cache: InMemoryCache,
    songbird: Songbird,
    standby: Standby,
}

fn spawn(
    fut: impl Future<Output = Result<(), Box<dyn Error + Send + Sync + 'static>>> + Send + 'static,
) {
    tokio::spawn(async move {
        if let Err(why) = fut.await {
            tracing::debug!("handler error: {:?}", why);
        }
    });
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    dotenv().ok();

    // Initialize the tracing subscriber.
    tracing_subscriber::fmt::init();

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

    let mut stream = ShardEventStream::new(shards.iter_mut());
    loop {
        let event = match stream.next().await {
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

        if let Event::MessageCreate(msg) = event {
            if msg.guild_id.is_none() || !msg.content.starts_with('!') {
                continue;
            }

            match msg.content.split(' ').next() {
                Some("!join") => spawn(join(msg.0, Arc::clone(&state))),
                Some("!leave") => spawn(leave(msg.0, Arc::clone(&state))),
                Some("!play") => spawn(play(msg.0, Arc::clone(&state))),
                Some("!stop") => spawn(stop(msg.0, Arc::clone(&state))),
                _ => continue,
            }
        }
    }

    Ok(())
}

async fn join(msg: Message, state: State) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let user_id = msg.author.id;
    let guild_id = msg.guild_id.ok_or("No guild id attached to the message.")?;
    let channel_id = state
        .cache
        .voice_state(user_id, guild_id)
        .ok_or("Cannot get voice state for user")?
        .channel_id();
    let channel_id =
        NonZeroU64::new(channel_id.into()).ok_or("Joined voice channel must have nonzero ID.")?;
    state
        .songbird
        .join(guild_id, channel_id)
        .await
        .map_err(|e| format!("Could not join voice channel: {:?}", e))?;
    Ok(())
}

async fn leave(msg: Message, state: State) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    tracing::debug!(
        "leave command in channel {} by {}",
        msg.channel_id,
        msg.author.name
    );
    let guild_id = msg.guild_id.unwrap();
    state.songbird.leave(guild_id).await?;
    Ok(())
}

async fn play(msg: Message, state: State) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    tracing::debug!(
        "play command in channel {} by {}",
        msg.channel_id,
        msg.author.name
    );

    join(msg.clone(), state.clone()).await?;

    let guild_id = msg.guild_id.unwrap();

    let url = msg.content.replace("!play", "").trim().to_string();
    let mut src = YoutubeDl::new(reqwest::Client::new(), url);
    if let Ok(metadata) = src.aux_metadata().await {
        debug!("metadata: {:?}", metadata);

        state
            .http
            .create_message(msg.channel_id)
            .content(&format!(
                "Playing **{:?}** by **{:?}**",
                metadata.title.as_ref().unwrap_or(&"<UNKNOWN>".to_string()),
                metadata.artist.as_ref().unwrap_or(&"<UNKNOWN>".to_string()),
            ))?
            .await?;

        if let Some(call_lock) = state.songbird.get(guild_id) {
            let mut call = call_lock.lock().await;
            let _handle = call.enqueue_with_preload(
                src.into(),
                metadata.duration.map(|duration| -> Duration {
                    if duration.as_secs() > 5 {
                        duration.sub(Duration::from_secs(5))
                    } else {
                        duration
                    }
                }),
            );
        }
    } else {
        state
            .http
            .create_message(msg.channel_id)
            .content("Didn't find any results")?
            .await?;
    }

    Ok(())
}

async fn stop(msg: Message, state: State) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    tracing::debug!(
        "stop command in channel {} by {}",
        msg.channel_id,
        msg.author.name
    );

    let guild_id = msg.guild_id.unwrap();

    if let Some(call_lock) = state.songbird.get(guild_id) {
        let mut call = call_lock.lock().await;
        call.stop();
    }

    state
        .http
        .create_message(msg.channel_id)
        .content("Stopped the track")?
        .await?;

    Ok(())
}

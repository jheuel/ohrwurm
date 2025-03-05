use crate::metadata::Metadata;
use crate::state::{State, StateRef};
use async_trait::async_trait;
use songbird::tracks::Track;
use songbird::{Event, EventContext, EventHandler, TrackEvent};
use std::ops::Sub;
use std::time::Duration;
use std::{error::Error, sync::Arc};
use twilight_model::{
    gateway::payload::incoming::InteractionCreate,
    http::interaction::{InteractionResponse, InteractionResponseType},
    id::{marker::GuildMarker, Id},
};
use twilight_util::builder::InteractionResponseDataBuilder;

pub(crate) async fn loop_queue(
    interaction: Box<InteractionCreate>,
    state: State,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    tracing::debug!(
        "loop command in guild {:?} in channel {:?} by {:?}",
        interaction.guild_id,
        interaction.channel,
        interaction.author(),
    );

    let guild_id: Id<GuildMarker> = if let Some(guild_id) = interaction.guild_id {
        guild_id
    } else {
        return Ok(());
    };

    state.guild_settings.entry(guild_id).and_modify(|settings| {
        settings.loop_queue = !settings.loop_queue;
    });

    let looping = state
        .guild_settings
        .get(&guild_id)
        .expect("Cannot get loop state")
        .loop_queue;

    if let Some(call_lock) = state.songbird.get(guild_id) {
        let mut call = call_lock.lock().await;
        call.add_global_event(
            Event::Track(TrackEvent::End),
            TrackEndNotifier {
                guild_id,
                state: Arc::clone(&state),
            },
        );
    }

    let message = if looping {
        "I'm now looping the current queue!".to_string()
    } else {
        "I'm not looping anymore!".to_string()
    };

    let interaction_response_data = InteractionResponseDataBuilder::new()
        .content(message)
        .build();

    let response = InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(interaction_response_data),
    };

    state
        .http
        .interaction(interaction.application_id)
        .create_response(interaction.id, &interaction.token, &response)
        .await?;

    Ok(())
}

struct TrackEndNotifier {
    guild_id: Id<GuildMarker>,
    state: Arc<StateRef>,
}

#[async_trait]
impl EventHandler for TrackEndNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if !self
            .state
            .guild_settings
            .get(&self.guild_id)
            .unwrap()
            .loop_queue
        {
            return None;
        }
        let EventContext::Track(track_list) = ctx else {
            return None;
        };
        let (_, track_handle) = track_list.first()?;
        if let Some(call_lock) = self.state.songbird.get(self.guild_id) {
            let mut call = call_lock.lock().await;

            // get metadata from finished track
            let old_metadata = track_handle.data::<Metadata>();
            // enqueue track
            let track = Track::new_with_data(old_metadata.src.clone().into(), old_metadata.clone());
            let _handle = call.enqueue_with_preload(
                track,
                old_metadata.duration.map(|duration| -> Duration {
                    if duration.as_secs() > 5 {
                        duration.sub(Duration::from_secs(5))
                    } else {
                        duration
                    }
                }),
            );
        }
        None
    }
}

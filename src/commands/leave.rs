use crate::state::{State, StateRef};
use anyhow::Context;
use std::{error::Error, sync::Arc};
use twilight_model::{
    gateway::payload::incoming::InteractionCreate,
    id::{marker::GuildMarker, Id},
};

pub(crate) async fn leave_if_alone(
    guild_id: Id<GuildMarker>,
    state: State,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let user = state
        .cache
        .current_user()
        .context("Cannot get current user")?;
    let user_voice_state = state
        .cache
        .voice_state(user.id, guild_id)
        .context("Cannot get voice state")?;
    let channel = state
        .cache
        .channel(user_voice_state.channel_id())
        .context("Cannot get channel")?;
    let channel_voice_states = state
        .cache
        .voice_channel_states(channel.id)
        .context("Cannot get voice channel")?;
    let count = channel_voice_states.count();

    // count is 1 if the bot is the only one in the channel
    if count == 1 {
        leave_channel(guild_id, Arc::clone(&state)).await?;
    }
    Ok(())
}

pub(crate) async fn leave_channel(
    guild_id: Id<GuildMarker>,
    state: Arc<StateRef>,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    // stop playing
    if let Some(call_lock) = state.songbird.get(guild_id) {
        let call = call_lock.lock().await;
        call.queue().stop();
    }
    // leave the voice channel
    state.songbird.leave(guild_id).await?;

    // reset guild settings
    state.guild_settings.remove(&guild_id);
    Ok(())
}

pub(crate) async fn leave(
    interaction: Box<InteractionCreate>,
    state: State,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    tracing::debug!(
        "leave command n guild {:?} in channel {:?} by {:?}",
        interaction.guild_id,
        interaction.channel,
        interaction.author(),
    );

    let Some(guild_id) = interaction.guild_id else {
        return Ok(());
    };

    leave_channel(guild_id, Arc::clone(&state)).await?;

    Ok(())
}

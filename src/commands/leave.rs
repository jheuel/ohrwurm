use crate::state::{State, StateRef};
use std::{error::Error, sync::Arc};
use twilight_model::{
    gateway::payload::incoming::InteractionCreate,
    id::{marker::GuildMarker, Id},
};

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

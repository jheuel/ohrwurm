use std::{error::Error, num::NonZeroU64};
use twilight_model::channel::Message;

use crate::state::State;

pub(crate) async fn join(
    msg: Message,
    state: State,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let guild_id = msg.guild_id.ok_or("No guild id attached to the message.")?;
    let user_id = msg.author.id;
    let channel_id = state
        .cache
        .voice_state(user_id, guild_id)
        .ok_or("Cannot get voice state for user")?
        .channel_id();
    let channel_id =
        NonZeroU64::new(channel_id.into()).ok_or("Joined voice channel must have nonzero ID.")?;

    // join the voice channel
    state
        .songbird
        .join(guild_id, channel_id)
        .await
        .map_err(|e| format!("Could not join voice channel: {:?}", e))?;

    // signal that we are not listening
    if let Some(call_lock) = state.songbird.get(guild_id) {
        let mut call = call_lock.lock().await;
        call.deafen(true).await?;
    }

    Ok(())
}

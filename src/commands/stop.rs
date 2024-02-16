use crate::state::State;
use std::error::Error;
use twilight_model::channel::Message;

pub(crate) async fn stop(
    msg: Message,
    state: State,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    tracing::debug!(
        "stop command in channel {} by {}",
        msg.channel_id,
        msg.author.name
    );

    let guild_id = msg.guild_id.unwrap();

    if let Some(call_lock) = state.songbird.get(guild_id) {
        let call = call_lock.lock().await;
        call.queue().stop();
    }

    state
        .http
        .create_message(msg.channel_id)
        .content("Stopped the track and cleared the queue")?
        .await?;

    Ok(())
}

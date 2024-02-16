use crate::state::State;
use std::error::Error;
use twilight_model::channel::Message;

pub(crate) async fn leave(
    msg: Message,
    state: State,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    tracing::debug!(
        "leave command in channel {} by {}",
        msg.channel_id,
        msg.author.name
    );
    let guild_id = msg.guild_id.unwrap();
    state.songbird.leave(guild_id).await?;
    Ok(())
}

use crate::{metadata::MetadataMap, state::State};
use std::error::Error;
use twilight_model::channel::Message;

pub(crate) async fn queue(
    msg: Message,
    state: State,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    tracing::debug!(
        "queue command in channel {} by {}",
        msg.channel_id,
        msg.author.name
    );
    let guild_id = msg.guild_id.unwrap();

    if let Some(call_lock) = state.songbird.get(guild_id) {
        let call = call_lock.lock().await;
        let queue = call.queue().current_queue();
        let mut message = String::new();
        if queue.is_empty() {
            message.push_str("There are no tracks in the queue.\n");
        } else {
            message.push_str("Currently playing:\n");
        }
        for track in queue {
            let map = track.typemap().read().await;
            let metadata = map.get::<MetadataMap>().unwrap();
            message.push_str(
                format!(
                    "* `{}",
                    metadata.title.clone().unwrap_or("Unknown".to_string()),
                )
                .as_str(),
            );
            if let Some(duration) = metadata.duration {
                let res = duration.as_secs();
                let hours = res / (60 * 60);
                let res = res - hours * 60 * 60;
                let minutes = res / 60;
                let res = res - minutes * 60;
                let seconds = res;
                message.push_str(" (");
                if hours > 0 {
                    message.push_str(format!("{:02}:", hours).as_str());
                }
                message.push_str(format!("{:02}:{:02}", minutes, seconds).as_str());
                message.push(')');
            }
            message.push_str("`\n");
        }
        state
            .http
            .create_message(msg.channel_id)
            .content(&message)?
            .await?;
    }
    Ok(())
}

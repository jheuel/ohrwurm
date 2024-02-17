use std::{env, error::Error, num::NonZeroU64, time::Duration};
use tokio::time::sleep;
use tracing::info;
use twilight_model::{channel::Message, id::Id};

use crate::state::State;

pub(crate) async fn delete(
    msg: Message,
    state: State,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let admin = env::var("ADMIN")?.parse::<u64>()?;
    if msg.author.id != Id::from(NonZeroU64::new(admin).unwrap()) {
        return Ok(());
    }
    let n = msg
        .content
        .split(' ')
        .last()
        .unwrap()
        .parse::<u16>()
        .unwrap_or(1);
    if n > 100 {
        return Ok(());
    }
    let messages = state
        .http
        .channel_messages(msg.channel_id)
        .before(msg.id)
        .limit(n)?
        .await?
        .model()
        .await?;
    state.http.delete_message(msg.channel_id, msg.id).await?;
    for message in messages {
        info!("Delete message: {:?}: {:?}", message.author.name, message);
        state
            .http
            .delete_message(msg.channel_id, message.id)
            .await?;
        sleep(Duration::from_secs(5)).await;
    }
    Ok(())
}

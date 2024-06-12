use crate::state::State;
use std::{env, error::Error, num::NonZeroU64, time::Duration};
use tokio::time::sleep;
use tracing::debug;
use twilight_model::{channel::Message, id::Id};

pub(crate) async fn delete(
    msg: Message,
    state: State,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let admin = env::var("ADMIN")?.parse::<u64>()?;
    if msg.author.id != Id::from(NonZeroU64::new(admin).expect("Could not get author id")) {
        return Ok(());
    }
    let n = msg
        .content
        .split(' ')
        .last()
        .unwrap_or("1")
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
        debug!("Delete message: {:?}: {:?}", message.author.name, message);
        state
            .http
            .delete_message(msg.channel_id, message.id)
            .await?;
        sleep(Duration::from_secs(5)).await;
    }
    Ok(())
}

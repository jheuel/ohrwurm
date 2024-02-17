use std::{env, error::Error, num::NonZeroU64, time::Duration};
use tokio::time::sleep;
use tracing::{debug, info};
use twilight_model::{
    application::interaction::Interaction,
    channel::message::MessageFlags,
    http::interaction::{InteractionResponse, InteractionResponseType},
    id::Id,
};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::state::State;

pub(crate) async fn delete(
    interaction: Interaction,
    state: State,
    count: i64,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    debug!(
        "delete command in guild {:?} in channel {:?} by {:?}",
        interaction.guild_id,
        interaction.channel,
        interaction.author(),
    );

    let admin = env::var("ADMIN")?.parse::<u64>()?;
    if interaction.author_id() != Some(Id::from(NonZeroU64::new(admin).unwrap())) {
        let interaction_response_data = InteractionResponseDataBuilder::new()
            .content("You do not have permissions to delete messages.")
            .flags(MessageFlags::EPHEMERAL)
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

        return Ok(());
    }
    let Some(channel) = interaction.channel else {
        return Ok(());
    };
    let Some(message_id) = channel.last_message_id else {
        return Ok(());
    };

    let count = count.max(1).min(100) as u16;

    let interaction_response_data = InteractionResponseDataBuilder::new()
        .content(format!("Deleting {count} messages."))
        .flags(MessageFlags::EPHEMERAL)
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

    let messages = state
        .http
        .channel_messages(channel.id)
        .before(message_id.cast())
        .limit(count)?
        .await?
        .model()
        .await?;
    for message in messages {
        debug!("Delete message: {:?}: {:?}", message.author.name, message);
        state.http.delete_message(channel.id, message.id).await?;
        sleep(Duration::from_secs(5)).await;
    }
    Ok(())
}

use crate::state::State;
use std::error::Error;
use twilight_model::{
    application::interaction::Interaction,
    channel::message::MessageFlags,
    http::interaction::{InteractionResponse, InteractionResponseType},
};
use twilight_util::builder::InteractionResponseDataBuilder;

pub(crate) async fn pause(
    interaction: Interaction,
    state: State,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    tracing::debug!(
        "pause command in guild {:?} in channel {:?} by {:?}",
        interaction.guild_id,
        interaction.channel,
        interaction.author(),
    );

    let Some(guild_id) = interaction.guild_id else {
        return Ok(());
    };

    if let Some(call_lock) = state.songbird.get(guild_id) {
        let call = call_lock.lock().await;
        call.queue().pause()?;
    }

    let interaction_response_data = InteractionResponseDataBuilder::new()
        .content("Paused the track")
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

    Ok(())
}

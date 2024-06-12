use crate::state::State;
use std::error::Error;
use tracing::debug;
use twilight_model::{
    channel::message::MessageFlags,
    gateway::payload::incoming::InteractionCreate,
    http::interaction::{InteractionResponse, InteractionResponseType},
    id::{
        marker::{GuildMarker, UserMarker},
        Id,
    },
};
use twilight_util::builder::InteractionResponseDataBuilder;

pub(crate) async fn join_channel(
    state: State,
    guild_id: Id<GuildMarker>,
    user_id: Id<UserMarker>,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    debug!("join user {:?} in guild {:?}", user_id, guild_id);

    let channel_id = state
        .cache
        .voice_state(user_id, guild_id)
        .ok_or("Cannot get voice state for user")?
        .channel_id();

    // join the voice channel
    state
        .songbird
        .join(guild_id.cast(), channel_id)
        .await
        .map_err(|e| format!("Could not join voice channel: {:?}", e))?;

    // signal that we are not listening
    if let Some(call_lock) = state.songbird.get(guild_id.cast()) {
        let mut call = call_lock.lock().await;
        call.deafen(true).await?;
    }

    Ok(())
}

pub(crate) async fn join(
    interaction: Box<InteractionCreate>,
    state: State,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    debug!(
        "join command in guild {:?} in channel {:?} by {:?}",
        interaction.guild_id,
        interaction.channel,
        interaction.author(),
    );

    let Some(guild_id) = interaction.guild_id else {
        return Ok(());
    };
    let Some(author_id) = interaction.author_id() else {
        return Ok(());
    };

    join_channel(state.clone(), guild_id, author_id).await?;

    let interaction_response_data = InteractionResponseDataBuilder::new()
        .content("Bin da Brudi!")
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

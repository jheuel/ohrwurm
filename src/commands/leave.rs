use crate::state::State;
use std::error::Error;
use twilight_model::gateway::payload::incoming::InteractionCreate;

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
    state.songbird.leave(guild_id).await?;
    Ok(())
}

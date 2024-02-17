use crate::state::State;
use std::error::Error;
use twilight_model::application::interaction::Interaction;

pub(crate) async fn leave(
    interaction: Interaction,
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

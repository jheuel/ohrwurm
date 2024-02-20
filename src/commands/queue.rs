use twilight_model::http::interaction::InteractionResponseType;
use twilight_model::{
    application::interaction::Interaction, channel::message::MessageFlags,
    http::interaction::InteractionResponse,
};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::{metadata::MetadataMap, state::State};
use std::error::Error;

pub(crate) async fn queue(
    interaction: Interaction,
    state: State,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    tracing::debug!(
        "queue command in guild {:?} in channel {:?} by {:?}",
        interaction.guild_id,
        interaction.channel,
        interaction.author(),
    );
    let Some(guild_id) = interaction.guild_id else {
        return Ok(());
    };
    if let Some(call_lock) = state.songbird.get(guild_id) {
        let call = call_lock.lock().await;
        let queue = call.queue().current_queue();
        let mut message = String::new();
        if queue.is_empty() {
            message.push_str("There are no tracks in the queue.\n");
        } else {
            message.push_str("Next songs are:\n");
        }
        for track in queue.iter().take(5) {
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

        let interaction_response_data = InteractionResponseDataBuilder::new()
            .content(&message)
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
    }
    Ok(())
}

use songbird::tracks::TrackHandle;
use twilight_model::channel::message::component::{ActionRow, Button, ButtonStyle};
use twilight_model::channel::message::{Component, Embed, MessageFlags, ReactionType};
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::http::interaction::InteractionResponse;
use twilight_model::http::interaction::InteractionResponseType;
use twilight_util::builder::embed::EmbedBuilder;
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::colors;
use crate::{metadata::MetadataMap, state::State};
use std::error::Error;

pub(crate) const TRACKS_PER_PAGE: usize = 5;

fn format_duration(duration: std::time::Duration) -> String {
    let res = duration.as_secs();
    let hours = res / (60 * 60);
    let res = res - hours * 60 * 60;
    let minutes = res / 60;
    let res = res - minutes * 60;
    let seconds = res;
    let mut s = String::new();
    if hours > 0 {
        s.push_str(format!("{:02}:", hours).as_str());
    }
    s.push_str(format!("{:02}:{:02}", minutes, seconds).as_str());
    s
}

pub(crate) async fn build_queue_embeds(queue: &[TrackHandle], page: usize) -> Vec<Embed> {
    let mut message = String::new();
    if queue.is_empty() {
        message.push_str("There are no tracks in the queue.\n");
    }
    for track in queue
        .iter()
        .skip(TRACKS_PER_PAGE * page)
        .take(TRACKS_PER_PAGE)
    {
        let map = track.typemap().read().await;
        let metadata = map
            .get::<MetadataMap>()
            .expect("Could not get metadata map");
        message.push_str(
            format!(
                "* [{}]({})",
                metadata.title.clone().unwrap_or("Unknown".to_string()),
                metadata.url,
            )
            .as_str(),
        );
        if let Some(duration) = metadata.duration {
            message.push_str(" (");
            message.push_str(&format_duration(duration));
            message.push(')');
        }
        message.push('\n');
    }
    message.push('\n');

    let max_pages = queue.len() / TRACKS_PER_PAGE;
    if max_pages > 0 {
        message.push_str(&format!("page {}/{}", 1 + page, 1 + max_pages));
    }
    vec![EmbedBuilder::new()
        .description(&message)
        .color(colors::BLURPLE)
        .build()]
}

pub(crate) fn build_action_row(page: usize, max_pages: usize) -> Vec<Component> {
    if max_pages == 0 {
        return Vec::new();
    }
    vec![Component::ActionRow(ActionRow {
        components: vec![
            Component::Button(Button {
                custom_id: Some(format!("page:{}", page as i32 - 1)),
                style: ButtonStyle::Primary,
                label: Some("Previous page".to_string()),
                emoji: Some(ReactionType::Unicode {
                    name: "⬅️".to_string(),
                }),
                url: None,
                disabled: page == 0,
            }),
            Component::Button(Button {
                custom_id: Some(format!("page:{}", page + 1)),
                style: ButtonStyle::Primary,
                label: Some("Next page".to_string()),
                emoji: Some(ReactionType::Unicode {
                    name: "➡️".to_string(),
                }),
                url: None,
                disabled: page >= max_pages,
            }),
        ],
    })]
}

pub(crate) async fn queue(
    interaction: Box<InteractionCreate>,
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

    let content = "Fetching queue".to_string();
    let embeds = vec![EmbedBuilder::new()
        .description(content)
        .color(colors::YELLOW)
        .build()];
    let interaction_response_data = InteractionResponseDataBuilder::new()
        .embeds(embeds)
        .flags(MessageFlags::LOADING)
        .build();
    let response = InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: Some(interaction_response_data),
    };
    state
        .http
        .interaction(interaction.application_id)
        .create_response(interaction.id, &interaction.token, &response)
        .await?;

    let mut queue = Vec::new();
    if let Some(call_lock) = state.songbird.get(guild_id) {
        let call = call_lock.lock().await;
        queue = call.queue().current_queue();
    }

    let embeds = build_queue_embeds(&queue, 0).await;
    let action_row = build_action_row(0, queue.len() / TRACKS_PER_PAGE);

    state
        .http
        .interaction(interaction.application_id)
        .update_response(&interaction.token)
        .embeds(Some(&embeds))?
        .components(Some(&action_row))?
        .await?;

    Ok(())
}

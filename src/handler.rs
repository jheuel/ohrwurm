use crate::commands::queue::{build_action_row, build_queue_embeds, TRACKS_PER_PAGE};
use crate::commands::{
    delete, join, leave, leave_if_alone, loop_queue, pause, play, queue, resume, skip, stop,
};
use crate::interaction_commands::InteractionCommand;
use crate::state::State;
use crate::utils::spawn;
use anyhow::Context;
use std::sync::Arc;
use twilight_gateway::Event;
use twilight_model::application::interaction::message_component::MessageComponentInteractionData;
use twilight_model::application::interaction::InteractionData;
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_util::builder::InteractionResponseDataBuilder;

pub(crate) struct Handler {
    state: State,
}

impl Handler {
    pub(crate) fn new(state: State) -> Self {
        Self { state }
    }
    pub(crate) async fn act(&self, event: Event) -> anyhow::Result<()> {
        self.handle_messages(&event).await?;
        self.handle_voice_state_update(&event).await?;
        self.handle_interaction(&event).await?;
        Ok(())
    }

    async fn handle_messages(&self, event: &Event) -> anyhow::Result<()> {
        match event {
            Event::MessageCreate(message) if message.content.starts_with('!') => {
                if message.content.contains("!delete") {
                    spawn(delete(message.0.clone(), Arc::clone(&self.state)));
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    async fn handle_voice_state_update(&self, event: &Event) -> anyhow::Result<()> {
        match event {
            Event::VoiceStateUpdate(update) => {
                let guild_id = update.guild_id.context("Guild ID not found")?;
                spawn(leave_if_alone(guild_id, Arc::clone(&self.state)));
                Ok(())
            }
            _ => Ok(()),
        }
    }

    async fn handle_interaction(&self, event: &Event) -> anyhow::Result<()> {
        match event {
            Event::InteractionCreate(interaction) => match &interaction.data {
                Some(InteractionData::ApplicationCommand(command)) => {
                    self.handle_application_command(command.clone().into(), interaction.clone())
                }
                Some(InteractionData::MessageComponent(data)) => {
                    self.handle_message_component(data, interaction.clone())
                        .await
                }
                _ => Ok(()),
            },
            _ => Ok(()),
        }
    }

    fn handle_application_command(
        &self,
        command: InteractionCommand,
        interaction: Box<InteractionCreate>,
    ) -> anyhow::Result<()> {
        {
            match command {
                InteractionCommand::Play(query) => {
                    spawn(play(interaction, Arc::clone(&self.state), query))
                }
                InteractionCommand::Stop => spawn(stop(interaction, Arc::clone(&self.state))),
                InteractionCommand::Pause => spawn(pause(interaction, Arc::clone(&self.state))),
                InteractionCommand::Skip => spawn(skip(interaction, Arc::clone(&self.state))),
                InteractionCommand::Loop => spawn(loop_queue(interaction, Arc::clone(&self.state))),
                InteractionCommand::Resume => spawn(resume(interaction, Arc::clone(&self.state))),
                InteractionCommand::Leave => spawn(leave(interaction, Arc::clone(&self.state))),
                InteractionCommand::Join => spawn(join(interaction, Arc::clone(&self.state))),
                InteractionCommand::Queue => spawn(queue(interaction, Arc::clone(&self.state))),
                _ => {}
            }
            Ok(())
        }
    }

    async fn handle_message_component(
        &self,
        data: &MessageComponentInteractionData,
        interaction: Box<InteractionCreate>,
    ) -> anyhow::Result<()> {
        if !data.custom_id.starts_with("page:") {
            return Ok(());
        }
        let page = data
            .custom_id
            .trim_start_matches("page:")
            .parse::<usize>()
            .unwrap_or(0);

        if let Some(guild_id) = interaction.guild_id {
            let mut queue = Vec::new();
            if let Some(call_lock) = self.state.songbird.get(guild_id) {
                let call = call_lock.lock().await;
                queue = call.queue().current_queue();
            }
            let n_pages = (queue.len() + TRACKS_PER_PAGE - 1) / TRACKS_PER_PAGE;
            let page = page.min(n_pages - 1).max(0);
            let embeds = build_queue_embeds(&queue, page).await;
            let action_row = build_action_row(page, n_pages);

            let interaction_response_data = InteractionResponseDataBuilder::new()
                .embeds(embeds)
                .components(action_row)
                .build();
            let response = InteractionResponse {
                kind: InteractionResponseType::UpdateMessage,
                data: Some(interaction_response_data),
            };
            self.state
                .http
                .interaction(interaction.application_id)
                .create_response(interaction.id, &interaction.token, &response)
                .await?;
            Ok(())
        } else {
            Ok(())
        }
    }
}

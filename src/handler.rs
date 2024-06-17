use crate::commands::queue::{build_action_row, build_queue_embeds, TRACKS_PER_PAGE};
use crate::commands::{delete, join, leave, loop_queue, pause, play, queue, resume, skip, stop};
use crate::state::State;
use futures::Future;
use std::error::Error;
use std::sync::Arc;
use tracing::debug;
use twilight_gateway::Event;
use twilight_model::application::interaction::application_command::{
    CommandData, CommandOptionValue,
};
use twilight_model::application::interaction::InteractionData;
use twilight_model::gateway::payload::incoming::VoiceStateUpdate;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_util::builder::InteractionResponseDataBuilder;

#[derive(Debug)]
enum InteractionCommand {
    Play(String),
    Stop,
    Pause,
    Skip,
    Loop,
    Resume,
    Leave,
    Join,
    Queue,
    NotImplemented,
}

fn spawn(
    fut: impl Future<Output = Result<(), Box<dyn Error + Send + Sync + 'static>>> + Send + 'static,
) {
    tokio::spawn(async move {
        if let Err(why) = fut.await {
            tracing::debug!("handler error: {:?}", why);
        }
    });
}

pub(crate) async fn leave_if_alone(
    update: VoiceStateUpdate,
    state: State,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let guild_id = update.guild_id.ok_or("Guild ID not found")?;
    let user = state
        .cache
        .current_user()
        .ok_or("Cannot get current user")?;
    let user_voice_state = state
        .cache
        .voice_state(user.id, guild_id)
        .ok_or("Cannot get voice state")?;
    let channel = state
        .cache
        .channel(user_voice_state.channel_id())
        .ok_or("Cannot get channel")?;
    let channel_voice_states = state
        .cache
        .voice_channel_states(channel.id)
        .ok_or("Cannot get voice channel")?;
    let count = channel_voice_states.count();

    // count is 1 if the bot is the only one in the channel
    if count == 1 {
        // stop playing
        if let Some(call_lock) = state.songbird.get(guild_id) {
            let call = call_lock.lock().await;
            call.queue().stop();
        }
        // leave the voice channel
        state.songbird.leave(guild_id).await?;
    }
    Ok(())
}

pub(crate) struct Handler {
    state: State,
}

impl Handler {
    pub(crate) fn new(state: State) -> Self {
        Self { state }
    }
    pub(crate) async fn act(&self, event: Event) -> anyhow::Result<()> {
        match event {
            Event::MessageCreate(message) if message.content.starts_with('!') => {
                if message.content.contains("!delete") {
                    spawn(delete(message.0.clone(), Arc::clone(&self.state)));
                }
                Ok(())
            }
            Event::VoiceStateUpdate(update) => {
                spawn(leave_if_alone(*update.clone(), Arc::clone(&self.state)));
                Ok(())
            }
            Event::InteractionCreate(interaction) => {
                tracing::info!("interaction: {:?}", &interaction);
                match &interaction.data {
                    Some(InteractionData::ApplicationCommand(command)) => {
                        let interaction_command = parse_interaction_command(command);
                        debug!("{:?}", interaction_command);
                        match interaction_command {
                            InteractionCommand::Play(query) => {
                                spawn(play(interaction, Arc::clone(&self.state), query))
                            }
                            InteractionCommand::Stop => {
                                spawn(stop(interaction, Arc::clone(&self.state)))
                            }
                            InteractionCommand::Pause => {
                                spawn(pause(interaction, Arc::clone(&self.state)))
                            }
                            InteractionCommand::Skip => {
                                spawn(skip(interaction, Arc::clone(&self.state)))
                            }
                            InteractionCommand::Loop => {
                                spawn(loop_queue(interaction, Arc::clone(&self.state)))
                            }
                            InteractionCommand::Resume => {
                                spawn(resume(interaction, Arc::clone(&self.state)))
                            }
                            InteractionCommand::Leave => {
                                spawn(leave(interaction, Arc::clone(&self.state)))
                            }
                            InteractionCommand::Join => {
                                spawn(join(interaction, Arc::clone(&self.state)))
                            }
                            InteractionCommand::Queue => {
                                spawn(queue(interaction, Arc::clone(&self.state)))
                            }
                            _ => {}
                        }
                        Ok(())
                    }
                    Some(InteractionData::MessageComponent(data)) => {
                        tracing::info!("message component: {:?}", data);

                        if !data.custom_id.starts_with("page:") {
                            return Ok(());
                        }
                        let page = data
                            .custom_id
                            .trim_start_matches("page:")
                            .parse::<usize>()
                            .unwrap_or(0);
                        tracing::info!("page: {:?}", page);

                        if let Some(guild_id) = interaction.guild_id {
                            let mut queue = Vec::new();
                            if let Some(call_lock) = self.state.songbird.get(guild_id) {
                                let call = call_lock.lock().await;
                                queue = call.queue().current_queue();
                            }
                            let embeds = build_queue_embeds(&queue, page).await;
                            let action_row = build_action_row(page, queue.len() / TRACKS_PER_PAGE);

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
                    _ => Ok(()),
                }
            }
            event => {
                tracing::info!("unhandled event: {:?}", event);
                Ok(())
            }
        }
    }
}

fn parse_interaction_command(command: &CommandData) -> InteractionCommand {
    debug!("command: {:?}", command);
    match command.name.as_str() {
        "play" => {
            if let Some(query_option) = command.options.iter().find(|opt| opt.name == "query") {
                if let CommandOptionValue::String(query) = &query_option.value {
                    InteractionCommand::Play(query.clone())
                } else {
                    InteractionCommand::NotImplemented
                }
            } else {
                InteractionCommand::NotImplemented
            }
        }
        "stop" => InteractionCommand::Stop,
        "pause" => InteractionCommand::Pause,
        "skip" => InteractionCommand::Skip,
        "loop" => InteractionCommand::Loop,
        "resume" => InteractionCommand::Resume,
        "leave" => InteractionCommand::Leave,
        "join" => InteractionCommand::Join,
        "queue" => InteractionCommand::Queue,
        _ => InteractionCommand::NotImplemented,
    }
}

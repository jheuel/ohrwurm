use crate::commands::{delete, join, leave, pause, play, queue, resume, stop};
use crate::state::State;
use futures::Future;
use std::error::Error;
use std::sync::Arc;
use tracing::debug;
use twilight_gateway::Event;
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::application::interaction::{Interaction, InteractionData};
use twilight_model::gateway::payload::incoming::VoiceStateUpdate;

#[derive(Debug)]
enum InteractionCommand {
    Play(Interaction, String),
    Stop(Interaction),
    Pause(Interaction),
    Resume(Interaction),
    Leave(Interaction),
    Join(Interaction),
    Queue(Interaction),
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
    pub(crate) async fn act(&self, event: Event) {
        match &event {
            Event::MessageCreate(message) if message.content.starts_with('!') => {
                if message.content.contains("!delete") {
                    spawn(delete(message.0.clone(), Arc::clone(&self.state)));
                }
            }
            Event::VoiceStateUpdate(update) => {
                spawn(leave_if_alone(*update.clone(), Arc::clone(&self.state)))
            }
            _ => {}
        }

        let interaction_command = match event {
            Event::InteractionCreate(interaction) => {
                debug!("interaction: {:?}", &interaction);
                match &interaction.data {
                    Some(InteractionData::ApplicationCommand(command)) => {
                        debug!("command: {:?}", command);
                        match command.name.as_str() {
                            "play" => {
                                if let Some(query_option) =
                                    command.options.iter().find(|opt| opt.name == "query")
                                {
                                    if let CommandOptionValue::String(query) = &query_option.value {
                                        InteractionCommand::Play(
                                            interaction.0.clone(),
                                            query.clone(),
                                        )
                                    } else {
                                        InteractionCommand::NotImplemented
                                    }
                                } else {
                                    InteractionCommand::NotImplemented
                                }
                            }
                            "stop" => InteractionCommand::Stop(interaction.0.clone()),
                            "pause" => InteractionCommand::Pause(interaction.0.clone()),
                            "resume" => InteractionCommand::Resume(interaction.0.clone()),
                            "leave" => InteractionCommand::Leave(interaction.0.clone()),
                            "join" => InteractionCommand::Join(interaction.0.clone()),
                            "queue" => InteractionCommand::Queue(interaction.0.clone()),
                            _ => InteractionCommand::NotImplemented,
                        }
                    }
                    _ => InteractionCommand::NotImplemented,
                }
            }
            _ => InteractionCommand::NotImplemented,
        };
        debug!("{:?}", interaction_command);
        match interaction_command {
            InteractionCommand::Play(interaction, query) => {
                spawn(play(interaction, Arc::clone(&self.state), query))
            }
            InteractionCommand::Stop(interaction) => {
                spawn(stop(interaction, Arc::clone(&self.state)))
            }
            InteractionCommand::Pause(interaction) => {
                spawn(pause(interaction, Arc::clone(&self.state)))
            }
            InteractionCommand::Resume(interaction) => {
                spawn(resume(interaction, Arc::clone(&self.state)))
            }
            InteractionCommand::Leave(interaction) => {
                spawn(leave(interaction, Arc::clone(&self.state)))
            }
            InteractionCommand::Join(interaction) => {
                spawn(join(interaction, Arc::clone(&self.state)))
            }
            InteractionCommand::Queue(interaction) => {
                spawn(queue(interaction, Arc::clone(&self.state)))
            }
            _ => {}
        };
    }
}

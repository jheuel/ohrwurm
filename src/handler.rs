use crate::commands::{join, leave, pause, play, queue, resume, stop};
use crate::state::State;

use futures::Future;
use std::error::Error;
use std::sync::Arc;

use twilight_gateway::Event;
use twilight_model::channel::Message;

enum ChatCommand {
    Play(Message, String),
    Stop(Message),
    Pause(Message),
    Resume(Message),
    Leave(Message),
    Join(Message),
    Queue(Message),
    NotImplemented,
}

fn parse_command(event: Event) -> Option<ChatCommand> {
    match event {
        Event::MessageCreate(msg_create) => {
            if msg_create.guild_id.is_none() || !msg_create.content.starts_with('!') {
                return None;
            }
            let split: Vec<&str> = msg_create.content.splitn(2, ' ').collect();
            match split.as_slice() {
                ["!play", query] => {
                    Some(ChatCommand::Play(msg_create.0.clone(), query.to_string()))
                }
                ["!stop"] | ["!stop", _] => Some(ChatCommand::Stop(msg_create.0)),
                ["!pause"] | ["!pause", _] => Some(ChatCommand::Pause(msg_create.0)),
                ["!resume"] | ["!resume", _] => Some(ChatCommand::Resume(msg_create.0)),
                ["!leave"] | ["!leave", _] => Some(ChatCommand::Leave(msg_create.0)),
                ["!join"] | ["!join", _] => Some(ChatCommand::Join(msg_create.0)),
                ["!queue"] | ["!queue", _] => Some(ChatCommand::Queue(msg_create.0)),
                _ => Some(ChatCommand::NotImplemented),
            }
        }
        _ => None,
    }
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

pub(crate) struct Handler {
    state: State,
}

impl Handler {
    pub(crate) fn new(state: State) -> Self {
        Self { state }
    }
    pub(crate) async fn act(&mut self, event: Event) {
        match parse_command(event) {
            Some(ChatCommand::Play(msg, query)) => spawn(play(msg, Arc::clone(&self.state), query)),
            Some(ChatCommand::Stop(msg)) => spawn(stop(msg, Arc::clone(&self.state))),
            Some(ChatCommand::Pause(msg)) => spawn(pause(msg, Arc::clone(&self.state))),
            Some(ChatCommand::Resume(msg)) => spawn(resume(msg, Arc::clone(&self.state))),
            Some(ChatCommand::Leave(msg)) => spawn(leave(msg, Arc::clone(&self.state))),
            Some(ChatCommand::Join(msg)) => spawn(join(msg, Arc::clone(&self.state))),
            Some(ChatCommand::Queue(msg)) => spawn(queue(msg, Arc::clone(&self.state))),
            _ => {}
        }
    }
}

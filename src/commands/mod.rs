mod join;
pub(crate) use join::join;

mod leave;
pub(crate) use leave::leave;

mod pause;
pub(crate) use pause::pause;

mod play;
pub(crate) use play::play;

mod queue;
pub(crate) use queue::queue;

mod resume;
pub(crate) use resume::resume;

mod stop;
pub(crate) use stop::stop;

mod delete;
pub(crate) use delete::delete;

use twilight_model::application::command::CommandType;
use twilight_util::builder::command::{CommandBuilder, IntegerBuilder, StringBuilder};

pub(crate) fn get_chat_commands() -> Vec<twilight_model::application::command::Command> {
    vec![
        CommandBuilder::new("join", "Join the channel", CommandType::ChatInput).build(),
        CommandBuilder::new("leave", "Leave the channel", CommandType::ChatInput).build(),
        CommandBuilder::new("pause", "Pause playing", CommandType::ChatInput).build(),
        CommandBuilder::new("play", "Add a song to the queue", CommandType::ChatInput)
            .option(StringBuilder::new("query", "URL of a song").required(true))
            .build(),
        CommandBuilder::new("queue", "Print track queue", CommandType::ChatInput).build(),
        CommandBuilder::new("resume", "Resume playing", CommandType::ChatInput).build(),
        CommandBuilder::new("stop", "Stop playing", CommandType::ChatInput).build(),
        CommandBuilder::new("delete", "Delete messages in chat", CommandType::ChatInput)
            .option(IntegerBuilder::new("count", "How many messages to delete").required(true))
            .build(),
    ]
}

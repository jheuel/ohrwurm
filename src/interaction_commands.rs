use twilight_model::application::interaction::application_command::{
    CommandData, CommandOptionValue,
};

#[derive(Debug)]
pub(crate) enum InteractionCommand {
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

impl From<Box<CommandData>> for InteractionCommand {
    fn from(command: Box<CommandData>) -> InteractionCommand {
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
}

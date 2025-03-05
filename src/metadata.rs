use songbird::input::YoutubeDl;
use std::time::Duration;

#[derive(Clone)]
pub(crate) struct Metadata {
    pub(crate) title: Option<String>,
    pub(crate) duration: Option<Duration>,
    pub(crate) url: String,
    pub(crate) src: YoutubeDl<'static>,
}

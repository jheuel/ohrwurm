use dashmap::DashMap;
use songbird::Songbird;
use std::sync::Arc;
use twilight_cache_inmemory::InMemoryCache;
use twilight_http::Client as HttpClient;
use twilight_model::id::{marker::GuildMarker, Id};
use twilight_standby::Standby;

pub(crate) type State = Arc<StateRef>;

#[derive(Debug)]
pub(crate) struct Settings {
    pub(crate) loop_queue: bool,
}

#[derive(Debug)]
pub(crate) struct StateRef {
    pub(crate) http: HttpClient,
    pub(crate) cache: InMemoryCache,
    pub(crate) songbird: Songbird,
    pub(crate) standby: Standby,
    pub(crate) guild_settings: DashMap<Id<GuildMarker>, Settings>,
}

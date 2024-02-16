use songbird::Songbird;
use std::sync::Arc;
use twilight_cache_inmemory::InMemoryCache;
use twilight_http::Client as HttpClient;
use twilight_standby::Standby;

pub(crate) type State = Arc<StateRef>;

#[derive(Debug)]
pub(crate) struct StateRef {
    pub(crate) http: HttpClient,
    pub(crate) cache: InMemoryCache,
    pub(crate) songbird: Songbird,
    pub(crate) standby: Standby,
}

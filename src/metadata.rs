use songbird::typemap::TypeMapKey;
use std::time::Duration;

pub(crate) struct Metadata {
    pub(crate) title: Option<String>,
    pub(crate) duration: Option<Duration>,
    pub(crate) url: String,
}

pub(crate) struct MetadataMap;
impl TypeMapKey for MetadataMap {
    type Value = Metadata;
}

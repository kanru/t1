use std::path::PathBuf;

use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct T1Config {
    pub(crate) t1bot: T1BotConfig,
    pub(crate) state_store: StateStoreConfig,
    pub(crate) rooms: RoomsConfig,
}

#[derive(Deserialize)]
pub(crate) struct T1BotConfig {
    pub(crate) user_id: String,
    pub(crate) password: String,
    pub(crate) display_name: String,
    pub(crate) device_id: String,
    pub(crate) device_name: String,
}

#[derive(Deserialize)]
pub(crate) struct StateStoreConfig {
    pub(crate) path: PathBuf,
    pub(crate) password: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct RoomsConfig {
    pub(crate) watching: Vec<String>,
}

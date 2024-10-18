use std::path::PathBuf;

use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct T1Config {
    pub(crate) t1bot: T1BotConfig,
    pub(crate) state_store: StateStoreConfig,
    pub(crate) rooms: RoomsConfig,
    pub(crate) captcha: CaptchaConfig,
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

#[derive(Deserialize)]
pub(crate) struct CaptchaConfig {
    pub(crate) timeout_secs: u64,
    pub(crate) questions: Vec<CaptchaQuestion>,
}

#[derive(Deserialize)]
pub(crate) struct CaptchaQuestion {
    pub(crate) body: String,
    pub(crate) answer: u8,
}

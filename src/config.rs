use std::{collections::HashMap, path::PathBuf};

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct T1Config {
    pub(crate) t1bot: T1BotConfig,
    pub(crate) state_store: StateStoreConfig,
    pub(crate) monitors: MonitorConfig,
    pub(crate) rooms: HashMap<String, RoomConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct T1BotConfig {
    pub(crate) user_id: String,
    pub(crate) password: String,
    pub(crate) display_name: String,
    pub(crate) device_id: String,
    pub(crate) device_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct StateStoreConfig {
    pub(crate) path: PathBuf,
    pub(crate) password: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MonitorConfig {
    pub(crate) rate_limit: Option<RateLimitConfig>,
    pub(crate) link_spam: Option<LinkSpamConfig>,
    pub(crate) captcha: Option<CaptchaConfig>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub(crate) enum RoomConfig {
    RoomEnabled(bool),
    RoomDetail {
        enabled: bool,
        monitors: MonitorConfig,
    },
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct RateLimitConfig {
    pub(crate) token_new: f32,
    pub(crate) token_new_max: f32,
    pub(crate) token_new_timeout_secs: u64,
    pub(crate) token_join: f32,
    pub(crate) token_join_max: f32,
    pub(crate) fill_rate: f32,
    pub(crate) fill_freq_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct LinkSpamConfig {
    pub(crate) watch_timeout_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CaptchaConfig {
    pub(crate) timeout_secs: u64,
    #[serde(default)]
    pub(crate) questions: Vec<CaptchaQuestion>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CaptchaQuestion {
    pub(crate) body: String,
    pub(crate) answer: u8,
}

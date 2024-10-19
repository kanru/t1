use ractor::{concurrency::Duration, Actor, ActorProcessingErr, ActorRef};

use crate::{
    actors::{
        config_provider::ConfigProviderMessage,
        moderator::{ModeratorMessage, ViolationKind},
    },
    config::{RateLimitConfig, RoomConfig},
    matrix::UserRoomId,
};

use super::MonitorMessage;

pub(super) struct RateLimitMonitor;

pub(super) struct RateLimitState {
    user_room_id: UserRoomId,
    bucket: Bucket,
    config: RateLimitConfig,
}

impl Actor for RateLimitMonitor {
    type State = RateLimitState;
    type Msg = MonitorMessage;
    type Arguments = UserRoomId;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(RateLimitState {
            user_room_id: args,
            bucket: Bucket::new(),
            config: Default::default(),
        })
    }

    async fn post_start(
        &self,
        myself: ActorRef<Self::Msg>,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        if let Some(config_provider) =
            ActorRef::<ConfigProviderMessage>::where_is("config_provider".into())
        {
            let config = config_provider
                .call(ConfigProviderMessage::GetConfig, None)
                .await?
                .unwrap();

            if let Some(rate_limit) = config
                .rooms
                .get(state.user_room_id.room_id.as_str())
                .and_then(|room| match room {
                    // FIXME: respect enabled
                    RoomConfig::RoomEnabled(_) => None,
                    RoomConfig::RoomDetail {
                        enabled: _,
                        monitors,
                    } => monitors.rate_limit.clone(),
                })
                .or(config.monitors.rate_limit)
            {
                state.bucket.token_current = rate_limit.token_new;
                state.bucket.token_max = rate_limit.token_new_max;
                state.bucket.fill_rate = rate_limit.fill_rate;
                state.bucket.fill_freq = Duration::from_secs(rate_limit.fill_freq_secs);
                state.config = rate_limit;
                myself.send_after(
                    Duration::from_secs(state.config.token_new_timeout_secs),
                    || MonitorMessage::Heartbeat,
                );
            } else {
                myself.stop(Some("disabled".into()));
            }
        }
        Ok(())
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            MonitorMessage::Heartbeat => {
                state.bucket.fill(state.bucket.fill_rate);
                if state.bucket.token_max == state.config.token_new_max {
                    state.bucket.token_max = state.config.token_join_max;
                }
                myself.send_after(state.bucket.fill_freq, || MonitorMessage::Heartbeat);
            }
            MonitorMessage::RoomMessage(_) | MonitorMessage::ReactionMessage(_) => {
                if !state.bucket.consume(1.0) {
                    if let Some(moderator) =
                        ActorRef::<ModeratorMessage>::where_is("moderator".into())
                    {
                        moderator.cast(ModeratorMessage::Violation {
                            user_room_id: state.user_room_id.clone(),
                            kind: ViolationKind::Spam,
                        })?;
                    } else {
                        tracing::error!("Unable to find moderator");
                    }
                }
            }
        };
        Ok(())
    }
}

#[derive(Debug)]
pub(super) struct Bucket {
    token_current: f32,
    token_max: f32,
    fill_rate: f32,
    fill_freq: Duration,
}

impl Bucket {
    fn new() -> Bucket {
        Bucket {
            token_current: 3.0,
            token_max: 3.0,
            fill_rate: 3.0,
            fill_freq: Duration::from_secs(60),
        }
    }

    fn consume(&mut self, count: f32) -> bool {
        if self.token_current < 0.0 {
            return false;
        }
        self.token_current -= count;
        self.token_current >= 0.0
    }

    fn fill(&mut self, count: f32) {
        self.token_current = f32::min(self.token_max, self.token_current + count);
    }
}

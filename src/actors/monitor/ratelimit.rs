use ractor::{concurrency::Duration, Actor, ActorProcessingErr, ActorRef};

use crate::{
    actors::moderator::{ModeratorMessage, ViolationKind},
    matrix::UserRoomId,
};

use super::MonitorMessage;

pub(super) struct RateLimitMonitor;

pub(super) struct RateLimitState {
    user_room_id: UserRoomId,
    bucket: Bucket,
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
        })
    }

    async fn post_start(
        &self,
        myself: ActorRef<Self::Msg>,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        myself.send_after(state.bucket.fill_freq, || MonitorMessage::Heartbeat);
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
                if state.bucket.is_new() {
                    state.bucket.upgrade();
                }
                myself.send_after(state.bucket.fill_freq, || MonitorMessage::Heartbeat);
            }
            MonitorMessage::RoomMessage(_) | MonitorMessage::ReactionMessage(_) => {
                if !state.bucket.consume(1) {
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
    token_current: i32,
    token_max: i32,
    fill_rate: i32,
    fill_freq: Duration,
}

impl Bucket {
    fn new() -> Bucket {
        Bucket {
            token_current: 3,
            token_max: 3,
            fill_rate: 3,
            fill_freq: Duration::from_secs(60),
        }
    }

    fn upgrade(&mut self) {
        self.token_max = 30;
        self.fill_rate = 10;
        self.fill_freq = Duration::from_secs(60);
    }

    fn is_new(&self) -> bool {
        self.token_max == 3
    }

    fn consume(&mut self, count: i32) -> bool {
        if self.token_current < 0 {
            return false;
        }
        self.token_current -= count;
        self.token_current >= 0
    }

    fn fill(&mut self, count: i32) {
        self.token_current = i32::min(self.token_max, self.token_current + count);
    }
}

use captcha::{CaptchaInit, CaptchaMonitor};
use link_spam::LinkSpamMonitor;
use matrix_sdk::{
    ruma::events::{reaction::SyncReactionEvent, room::message::SyncRoomMessageEvent},
    Client,
};
use ractor::{concurrency::Duration, pg, Actor, ActorProcessingErr, ActorRef};
use ratelimit::RateLimitMonitor;

use crate::matrix::UserRoomId;

mod captcha;
mod link_spam;
mod ratelimit;

const MONITOR_EXPIRE_TIMEOUT: u64 = 60 * 24;

#[derive(Debug, Clone)]
pub(crate) enum MonitorMessage {
    Heartbeat,
    RoomMessage(SyncRoomMessageEvent),
    ReactionMessage(SyncReactionEvent),
}

pub(crate) struct MonitorState {
    age: u64,
    last_msg_age: u64,
}

pub(crate) enum MonitorInit {
    Msg,
    Join,
}

pub(crate) struct Monitor;

impl Actor for Monitor {
    type State = MonitorState;
    type Msg = MonitorMessage;
    type Arguments = (UserRoomId, Client, MonitorInit);

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        let mut monitors = vec![];
        let (ratelimit, _) =
            Actor::spawn_linked(None, RateLimitMonitor, args.0.clone(), myself.get_cell()).await?;
        monitors.push(ratelimit.get_cell());
        let (link_spam, _) =
            Actor::spawn_linked(None, LinkSpamMonitor, args.0.clone(), myself.get_cell()).await?;
        monitors.push(link_spam.get_cell());
        if matches!(args.2, MonitorInit::Join) {
            let (captcha, _) = Actor::spawn_linked(
                None,
                CaptchaMonitor,
                CaptchaInit {
                    user_room_id: args.0.clone(),
                    client: args.1.clone(),
                },
                myself.get_cell(),
            )
            .await?;
            monitors.push(captcha.get_cell());
        }
        pg::join(myself.get_id().to_string(), monitors);
        Ok(MonitorState {
            age: 0,
            last_msg_age: 0,
        })
    }

    async fn post_start(
        &self,
        myself: ActorRef<Self::Msg>,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        myself.send_interval(Duration::from_secs(60), || MonitorMessage::Heartbeat);
        Ok(())
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        let sub_monitors = pg::get_members(&myself.get_id().to_string());
        match message {
            MonitorMessage::Heartbeat => {
                state.age += 1;
                if state.age - state.last_msg_age > MONITOR_EXPIRE_TIMEOUT {
                    myself.stop(Some("idled too long".into()));
                }
            }
            msg @ MonitorMessage::RoomMessage(_) => {
                for mon in sub_monitors {
                    ActorRef::<MonitorMessage>::from(mon).cast(msg.clone())?;
                }
                state.last_msg_age = state.age;
            }
            msg @ MonitorMessage::ReactionMessage(_) => {
                for mon in sub_monitors {
                    ActorRef::<MonitorMessage>::from(mon).cast(msg.clone())?;
                }
                state.last_msg_age = state.age;
            }
        };
        Ok(())
    }

    async fn handle_supervisor_evt(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: ractor::SupervisionEvent,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            ractor::SupervisionEvent::ActorStarted(actor_cell) => {
                tracing::info!(actor = actor_cell.get_id().to_string(), "Actor started")
            }
            ractor::SupervisionEvent::ActorTerminated(actor_cell, _, reason) => {
                tracing::info!(
                    actor = actor_cell.get_id().to_string(),
                    reason = reason,
                    "Actor stopped"
                );
            }
            ractor::SupervisionEvent::ActorFailed(actor_cell, error) => {
                tracing::info!(
                    actor = actor_cell.get_id().to_string(),
                    error = error,
                    "Actor failed"
                );
                return Err(error);
            }
            _ => {}
        };
        Ok(())
    }
}

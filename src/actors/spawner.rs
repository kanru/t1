use matrix_sdk::Client;
use ractor::{registry, Actor, ActorProcessingErr, ActorRef, SupervisionEvent};
use tracing::{error, info};

use crate::matrix::UserRoomId;

use super::monitor::{Monitor, MonitorInit};

pub(crate) struct Spawner;

pub(crate) enum SpawnerMessage {
    RegisterUser(UserRoomId),
    RegisterUserJoin(UserRoomId),
}

impl Actor for Spawner {
    type Msg = SpawnerMessage;
    type State = Client;
    type Arguments = Client;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(args)
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            SpawnerMessage::RegisterUser(user_room_id) => {
                if registry::where_is(user_room_id.to_string()).is_none() {
                    Actor::spawn_linked(
                        Some(user_room_id.to_string()),
                        Monitor,
                        (user_room_id, state.clone(), MonitorInit::Msg),
                        myself.into(),
                    )
                    .await?;
                }
            }
            SpawnerMessage::RegisterUserJoin(user_room_id) => {
                if registry::where_is(user_room_id.to_string()).is_none() {
                    Actor::spawn_linked(
                        Some(user_room_id.to_string()),
                        Monitor,
                        (user_room_id, state.clone(), MonitorInit::Join),
                        myself.into(),
                    )
                    .await?;
                }
            }
        };

        Ok(())
    }

    async fn handle_supervisor_evt(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: SupervisionEvent,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            SupervisionEvent::ActorStarted(actor_cell) => {
                info!(actor = actor_cell.get_name(), "Actor started");
            }
            SupervisionEvent::ActorTerminated(actor_cell, _, reason) => {
                info!(
                    actor = actor_cell.get_name(),
                    reason = reason,
                    "Actor stopped"
                );
            }
            SupervisionEvent::ActorFailed(actor_cell, error) => {
                error!("{error:?}");
                error!(actor = actor_cell.get_id().to_string(), "Actor failed");
            }
            _ => {}
        };
        Ok(())
    }
}

use ractor::{registry, Actor, ActorProcessingErr, ActorRef, SupervisionEvent};

use crate::matrix::UserRoomId;

use super::monitor::Monitor;

pub(crate) struct Spawner;

pub(crate) enum SpawnerMessage {
    RegisterUser(UserRoomId),
    RegisterUserJoin(UserRoomId),
}

impl Actor for Spawner {
    type State = ();
    type Msg = SpawnerMessage;
    type Arguments = ();

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _arguments: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(())
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            SpawnerMessage::RegisterUser(user_room_id) => {
                if registry::where_is(user_room_id.to_string()).is_none() {
                    Actor::spawn_linked(
                        Some(user_room_id.to_string()),
                        Monitor,
                        user_room_id,
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
                        user_room_id,
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
                tracing::info!(actor = actor_cell.get_name(), "Actor started");
            }
            SupervisionEvent::ActorTerminated(actor_cell, _, reason) => {
                tracing::info!(
                    actor = actor_cell.get_name(),
                    reason = reason,
                    "Actor stopped"
                );
            }
            SupervisionEvent::ActorFailed(actor_cell, error) => {
                tracing::error!(
                    actor = actor_cell.get_id().to_string(),
                    error = error,
                    "Actor failed"
                );
            }
            _ => {}
        };
        Ok(())
    }
}

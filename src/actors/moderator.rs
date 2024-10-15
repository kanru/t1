use matrix_sdk::Client;
use ractor::{Actor, ActorProcessingErr, ActorRef};

use crate::matrix::UserRoomId;

// TODO user real user_id and room_id type
pub(crate) enum ModeratorMessage {
    Ban {
        user_room_id: UserRoomId,
        reason: Option<&'static str>,
    },
    Kick {
        user_room_id: UserRoomId,
        reason: Option<&'static str>,
    },
}

pub(crate) struct Moderator;

impl Actor for Moderator {
    type Msg = ModeratorMessage;
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
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            ModeratorMessage::Ban {
                user_room_id,
                reason,
            } => {
                if let Some(room) = state.get_room(&user_room_id.room_id) {
                    tracing::info!(
                        "Baning user {} from {} for {}",
                        user_room_id.user_id,
                        user_room_id.room_id,
                        reason.unwrap_or("unknown reason")
                    );
                    room.ban_user(&user_room_id.user_id, reason).await?;
                }
            }
            ModeratorMessage::Kick {
                user_room_id,
                reason,
            } => {
                if let Some(room) = state.get_room(&user_room_id.room_id) {
                    tracing::info!(
                        "Kicking user {} from {} for {}",
                        user_room_id.user_id,
                        user_room_id.room_id,
                        reason.unwrap_or("unknown reason")
                    );
                    room.kick_user(&user_room_id.user_id, reason).await?;
                }
            }
        };
        Ok(())
    }
}

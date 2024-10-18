use matrix_sdk::Client;
use ractor::{Actor, ActorProcessingErr, ActorRef};

use crate::{actors::monitor::Monitor, matrix::UserRoomId};

#[derive(Debug)]
pub(crate) enum ViolationKind {
    Spam,
    LikelyBot,
}

// TODO user real user_id and room_id type
pub(crate) enum ModeratorMessage {
    Violation {
        user_room_id: UserRoomId,
        kind: ViolationKind,
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
            ModeratorMessage::Violation { user_room_id, kind } => {
                if let Some(room) = state.get_room(&user_room_id.room_id) {
                    tracing::info!(
                        "Kicking user {} from {} for {:?}",
                        user_room_id.user_id,
                        user_room_id.room_id,
                        kind
                    );
                    room.kick_user(&user_room_id.user_id, Some(format!("{:?}", kind).as_str()))
                        .await?;
                    // Stop the monitor
                    if let Some(monitor) = ActorRef::<Monitor>::where_is(user_room_id.to_string()) {
                        monitor.stop(Some("moderated".into()));
                    }
                }
            }
        };
        Ok(())
    }
}

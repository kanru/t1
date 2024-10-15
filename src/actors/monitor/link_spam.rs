use ractor::{concurrency::Duration, Actor, ActorRef};

use crate::{actors::moderator::ModeratorMessage, matrix::UserRoomId};

use super::MonitorMessage;

pub(super) struct LinkSpamMonitor;

pub(super) struct LinkSpamState {
    user_room_id: UserRoomId,
}

impl Actor for LinkSpamMonitor {
    type Msg = MonitorMessage;
    type State = LinkSpamState;
    type Arguments = UserRoomId;

    async fn pre_start(
        &self,
        myself: ractor::ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ractor::ActorProcessingErr> {
        myself.send_after(Duration::from_secs(10), || MonitorMessage::Heartbeat);
        Ok(LinkSpamState { user_room_id: args })
    }

    async fn handle(
        &self,
        myself: ractor::ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ractor::ActorProcessingErr> {
        match message {
            MonitorMessage::Heartbeat => {
                myself.stop(Some("waited long enough".into()));
            }
            MonitorMessage::RoomMessage(sync_message_like_event) => {
                if let Some(evt) = sync_message_like_event.as_original() {
                    if evt.content.body().contains("https://")
                        || evt.content.body().contains("http://")
                    {
                        if let Some(moderator) =
                            ActorRef::<ModeratorMessage>::where_is("moderator".into())
                        {
                            moderator.cast(ModeratorMessage::Kick {
                                user_room_id: state.user_room_id.clone(),
                                reason: Some("spam"),
                            })?;
                        } else {
                            tracing::error!("Unable to find moderator");
                        }
                    }
                }
            }
        };
        Ok(())
    }
}

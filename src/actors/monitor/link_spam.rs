use ractor::{concurrency::Duration, Actor, ActorRef};

use crate::{
    actors::{
        config_provider::ConfigProviderMessage,
        moderator::{ModeratorMessage, ViolationKind},
    },
    config::RoomConfig,
    matrix::UserRoomId,
};

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
        if let Some(config_provider) =
            ActorRef::<ConfigProviderMessage>::where_is("config_provider".into())
        {
            let config = config_provider
                .call(ConfigProviderMessage::GetConfig, None)
                .await?
                .unwrap();

            if let Some(link_spam) = config
                .rooms
                .get(args.room_id.as_str())
                .and_then(|room| match room {
                    // FIXME: respect enabled
                    RoomConfig::RoomEnabled(_) => None,
                    RoomConfig::RoomDetail {
                        enabled: _,
                        monitors,
                    } => monitors.link_spam.clone(),
                })
                .or(config.monitors.link_spam)
            {
                myself.send_after(Duration::from_secs(link_spam.watch_timeout_secs), || {
                    MonitorMessage::Heartbeat
                });
            } else {
                myself.stop(Some("disabled".to_string()));
            }
        }
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
                            moderator.cast(ModeratorMessage::Violation {
                                user_room_id: state.user_room_id.clone(),
                                kind: ViolationKind::Spam,
                            })?;
                        } else {
                            tracing::error!("Unable to find moderator");
                        }
                    }
                }
            }
            _ => {}
        };
        Ok(())
    }
}

use matrix_sdk::{
    ruma::{
        events::{
            reaction::ReactionEventContent, relation::Annotation,
            room::message::RoomMessageEventContent, Mentions,
        },
        OwnedEventId,
    },
    Client,
};
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

pub(super) struct CaptchaMonitor;

pub(super) struct CaptchaInit {
    pub(super) user_room_id: UserRoomId,
    pub(super) client: Client,
}

pub(super) struct CaptchaState {
    user_room_id: UserRoomId,
    client: Client,
    event_id: Option<OwnedEventId>,
    answer: String,
}

impl Actor for CaptchaMonitor {
    type Msg = MonitorMessage;
    type State = CaptchaState;
    type Arguments = CaptchaInit;

    async fn pre_start(
        &self,
        _myself: ractor::ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ractor::ActorProcessingErr> {
        Ok(CaptchaState {
            user_room_id: args.user_room_id,
            client: args.client,
            event_id: None,
            answer: "".to_string(),
        })
    }

    async fn post_start(
        &self,
        myself: ractor::ActorRef<Self::Msg>,
        state: &mut Self::State,
    ) -> Result<(), ractor::ActorProcessingErr> {
        if let Some(config_provider) =
            ActorRef::<ConfigProviderMessage>::where_is("config_provider".into())
        {
            let config = config_provider
                .call(ConfigProviderMessage::GetConfig, None)
                .await?
                .unwrap();

            if let Some(captcha) = config
                .rooms
                .get(state.user_room_id.room_id.as_str())
                .and_then(|room| match room {
                    RoomConfig::RoomEnabled(_) => None,
                    RoomConfig::RoomDetail {
                        enabled: _,
                        monitors,
                    } => monitors.captcha.clone(),
                })
                .or(config.monitors.captcha)
            {
                let choose = rand::random::<usize>();
                if let Some(question) = captcha.questions.get(choose % captcha.questions.len()) {
                    if let Some(room) = state.client.get_room(&state.user_room_id.room_id) {
                        let user = state
                            .client
                            .get_profile(&state.user_room_id.user_id)
                            .await?;
                        let display_name = user
                            .displayname
                            .unwrap_or(state.user_room_id.user_id.localpart().to_string());
                        let matrix_url = state.user_room_id.user_id.matrix_to_uri().to_string();
                        let body = format!("{display_name}: {}", question.body);
                        let html_body = format!(
                            "<a href='{matrix_url}'>{display_name}</a>: {}",
                            question.body
                        );
                        let content =
                            RoomMessageEventContent::notice_html(body, html_body).add_mentions(
                                Mentions::with_user_ids([state.user_room_id.user_id.clone()]),
                            );
                        let msg_response = room.send(content).await?;
                        let option1 = ReactionEventContent::new(Annotation::new(
                            msg_response.event_id.clone(),
                            "1️⃣".to_string(),
                        ));
                        let option2 = ReactionEventContent::new(Annotation::new(
                            msg_response.event_id.clone(),
                            "2️⃣".to_string(),
                        ));
                        let option3 = ReactionEventContent::new(Annotation::new(
                            msg_response.event_id.clone(),
                            "3️⃣".to_string(),
                        ));
                        let option4 = ReactionEventContent::new(Annotation::new(
                            msg_response.event_id.clone(),
                            "4️⃣".to_string(),
                        ));
                        room.send(option1).await?;
                        room.send(option2).await?;
                        room.send(option3).await?;
                        room.send(option4).await?;
                        state.event_id = Some(msg_response.event_id);
                        state.answer = match question.answer {
                            1 => "1️⃣",
                            2 => "2️⃣",
                            3 => "3️⃣",
                            4 => "4️⃣",
                            _ => "*️⃣",
                        }
                        .to_string();
                        myself.send_after(Duration::from_secs(captcha.timeout_secs), || {
                            MonitorMessage::Heartbeat
                        });
                    }
                }
            }
        }
        Ok(())
    }

    async fn handle(
        &self,
        myself: ractor::ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ractor::ActorProcessingErr> {
        match message {
            MonitorMessage::Heartbeat => {
                if let Some(moderator) = ActorRef::<ModeratorMessage>::where_is("moderator".into())
                {
                    moderator.cast(ModeratorMessage::Violation {
                        user_room_id: state.user_room_id.clone(),
                        kind: ViolationKind::LikelyBot,
                    })?;
                } else {
                    tracing::error!("Unable to find moderator");
                }
                if let Some(my_event_id) = &state.event_id {
                    if let Some(room) = state.client.get_room(&state.user_room_id.room_id) {
                        myself.stop(Some("answered".to_string()));
                        room.redact(&my_event_id, None, None).await?;
                    }
                }
            }
            MonitorMessage::ReactionMessage(msg) => {
                if let Some(msg) = msg.as_original() {
                    if let Some(my_event_id) = &state.event_id {
                        if msg.content.relates_to.event_id == *my_event_id {
                            if msg.content.relates_to.key != state.answer {
                                if let Some(moderator) =
                                    ActorRef::<ModeratorMessage>::where_is("moderator".into())
                                {
                                    moderator.cast(ModeratorMessage::Violation {
                                        user_room_id: state.user_room_id.clone(),
                                        kind: ViolationKind::LikelyBot,
                                    })?;
                                } else {
                                    tracing::error!("Unable to find moderator");
                                }
                            }
                            if let Some(room) = state.client.get_room(&state.user_room_id.room_id) {
                                myself.stop(Some("answered".to_string()));
                                room.redact(&my_event_id, None, None).await?;
                            }
                        }
                    }
                }
            }
            _ => {}
        };
        Ok(())
    }
}

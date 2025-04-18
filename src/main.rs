use std::{fs, path::PathBuf};

use actors::{
    monitor::MonitorMessage,
    spawner::SpawnerMessage,
    supervisor::{Supervisor, SupervisorState},
};
use config::T1Config;
use matrix::UserRoomId;
use matrix_sdk::{
    config::{RequestConfig, SyncSettings},
    ruma::{
        events::{
            reaction::SyncReactionEvent,
            room::{
                member::{MembershipState, SyncRoomMemberEvent},
                message::SyncRoomMessageEvent,
            },
        },
        MilliSecondsSinceUnixEpoch, RoomOrAliasId, UserId,
    },
    Client, Room,
};
use ractor::{Actor, ActorRef};
use tokio::{signal::unix::SignalKind, time::Duration};

mod actors;
mod config;
mod matrix;

const MAX_MESSAGE_DELAY_MS: u32 = 10_000;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let flags = xflags::parse_or_exit! {
        /// Path to the config file (TOML)
        required -c, --config config_path: PathBuf
    };

    let config_text = fs::read_to_string(&flags.config)?;
    let config: T1Config = toml::from_str(&config_text)?;

    let t1bot = UserId::parse(&config.t1bot.user_id)?;
    let client = Client::builder()
        .server_name(t1bot.server_name())
        .request_config(RequestConfig::short_retry())
        .sqlite_store(
            &config.state_store.path,
            config.state_store.password.as_deref(),
        )
        .build()
        .await?;

    client
        .matrix_auth()
        .login_username(&t1bot, &config.t1bot.password)
        .device_id(&config.t1bot.device_id)
        .initial_device_display_name(&config.t1bot.device_name)
        .send()
        .await?;

    client.sync_once(SyncSettings::default()).await?;

    client
        .account()
        .set_display_name(Some(config.t1bot.display_name.as_str()))
        .await?;

    let (supervisor, actor_handle) = Actor::spawn(
        Some("supervisor".into()),
        Supervisor,
        SupervisorState {
            client: client.clone(),
            config_path: flags.config.clone(),
        },
    )
    .await?;

    let my_id = t1bot.clone();
    client.add_event_handler(
        async move |ev: SyncRoomMessageEvent, room: Room| -> anyhow::Result<()> {
            if MilliSecondsSinceUnixEpoch::now()
                .get()
                .saturating_sub(ev.origin_server_ts().get())
                > MAX_MESSAGE_DELAY_MS.into()
            {
                tracing::info!(
                    origin_server_ts = i64::from(ev.origin_server_ts().0),
                    now = i64::from(MilliSecondsSinceUnixEpoch::now().0),
                    "Network latency increased - ignoring messages that are too old"
                );
                return Ok(());
            }
            if ev.sender().server_name().host() == "t2bot.io" {
                tracing::info!("Ignore messages from t2bot.io (Telegram Bridge)");
                return Ok(());
            }
            if ev.sender() == my_id {
                return Ok(());
            }

            let user_room_id = UserRoomId {
                user_id: ev.sender().into(),
                room_id: room.room_id().into(),
            };
            if let Some(monitor) = ActorRef::<MonitorMessage>::where_is(user_room_id.to_string()) {
                monitor.cast(MonitorMessage::RoomMessage(ev))?;
            } else if let Some(spawner) = ActorRef::<SpawnerMessage>::where_is("spawner".into()) {
                spawner.cast(SpawnerMessage::RegisterUser(user_room_id))?;
            }
            Ok(())
        },
    );

    let my_id = t1bot.clone();
    client.add_event_handler(
        async move |ev: SyncRoomMemberEvent, room: Room| -> anyhow::Result<()> {
            if let Some(ev) = ev.as_original() {
                if ev.state_key == my_id {
                    return Ok(());
                }
                if ev.state_key.server_name().host() == "t2bot.io" {
                    tracing::info!("Ignore messages from t2bot.io (Telegram Bridge)");
                    return Ok(());
                }
                let user_room_id = UserRoomId {
                    user_id: ev.state_key.clone(),
                    room_id: room.room_id().into(),
                };
                match ev.content.membership {
                    MembershipState::Join => {
                        if let Some(spawner) =
                            ActorRef::<SpawnerMessage>::where_is("spawner".into())
                        {
                            spawner.cast(SpawnerMessage::RegisterUserJoin(user_room_id))?;
                        }
                    }
                    MembershipState::Leave => {
                        if let Some(monitor) =
                            ActorRef::<MonitorMessage>::where_is(user_room_id.to_string())
                        {
                            monitor.stop(Some("leave".into()));
                        }
                    }
                    _ => {}
                }
            }
            Ok(())
        },
    );

    let my_id = t1bot.clone();
    client.add_event_handler(
        async move |ev: SyncReactionEvent, room: Room| -> anyhow::Result<()> {
            if MilliSecondsSinceUnixEpoch::now()
                .get()
                .saturating_sub(ev.origin_server_ts().get())
                > MAX_MESSAGE_DELAY_MS.into()
            {
                tracing::info!(
                    origin_server_ts = i64::from(ev.origin_server_ts().0),
                    now = i64::from(MilliSecondsSinceUnixEpoch::now().0),
                    "Network latency increased - ignoring messages that are too old"
                );
                return Ok(());
            }
            if ev.sender() == my_id {
                return Ok(());
            }
            let user_room_id = UserRoomId {
                user_id: ev.sender().into(),
                room_id: room.room_id().into(),
            };
            if let Some(monitor) = ActorRef::<MonitorMessage>::where_is(user_room_id.to_string()) {
                monitor.cast(MonitorMessage::ReactionMessage(ev))?;
            }
            Ok(())
        },
    );

    let server_names = &[t1bot.server_name().into()];
    for room_id in config.rooms.keys() {
        client
            .join_room_by_id_or_alias(&RoomOrAliasId::parse(room_id)?, server_names)
            .await?;
    }

    let mut sigterm = tokio::signal::unix::signal(SignalKind::terminate())?;
    let mut sigint = tokio::signal::unix::signal(SignalKind::interrupt())?;
    loop {
        tokio::select! {
            _ = sigterm.recv() => {
                tracing::info!("Received terminate signal, stopping the sync loop");
                break;
            }
            _ = sigint.recv() => {
                tracing::info!("Received interrupt signal, stopping the sync loop");
                break;
            }
            result = client.sync(SyncSettings::default()) => {
                match result {
                    Ok(()) => {
                        tracing::info!("Sync cancelled, stopping the sync loop");
                        break;
                    }
                    Err(err) => {
                        tracing::error!(error = err.to_string(), "Sync failed - restarting the sync loop");
                        tokio::time::sleep(Duration::from_secs(10)).await;
                    }
                };
            }
        }
    }

    supervisor.stop(None);
    actor_handle.await?;

    Ok(())
}

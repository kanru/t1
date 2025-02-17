use std::path::PathBuf;

use matrix_sdk::Client;
use ractor::{Actor, ActorProcessingErr, ActorRef, SupervisionEvent};
use tracing::{error, info};

use super::{config_provider::ConfigProvider, moderator::Moderator, spawner::Spawner};

pub(crate) struct Supervisor;

pub(crate) struct SupervisorState {
    pub(crate) client: Client,
    pub(crate) config_path: PathBuf,
}

pub(crate) enum SupervisorMessage {}

async fn start_spawner(myself: &ActorRef<SupervisorMessage>, client: Client) -> anyhow::Result<()> {
    Actor::spawn_linked(Some("spawner".into()), Spawner, client, myself.get_cell()).await?;
    Ok(())
}

async fn start_config_provider(
    myself: &ActorRef<SupervisorMessage>,
    config_path: PathBuf,
) -> anyhow::Result<()> {
    Actor::spawn_linked(
        Some("config_provider".into()),
        ConfigProvider,
        config_path,
        myself.get_cell(),
    )
    .await?;
    Ok(())
}

async fn start_moderator(
    myself: &ActorRef<SupervisorMessage>,
    client: Client,
) -> anyhow::Result<()> {
    Actor::spawn_linked(
        Some("moderator".into()),
        Moderator,
        client,
        myself.get_cell(),
    )
    .await?;
    Ok(())
}

impl Actor for Supervisor {
    type Msg = SupervisorMessage;
    type State = SupervisorState;
    type Arguments = SupervisorState;

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        start_spawner(&myself, args.client.clone()).await?;
        start_config_provider(&myself, args.config_path.clone()).await?;
        start_moderator(&myself, args.client.clone()).await?;

        Ok(args)
    }

    async fn handle_supervisor_evt(
        &self,
        myself: ActorRef<Self::Msg>,
        message: SupervisionEvent,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            SupervisionEvent::ActorStarted(actor_cell) => {
                info!(actor = actor_cell.get_name(), "Actor started");
            }
            SupervisionEvent::ActorTerminated(actor_cell, _, _) => {
                info!(actor = actor_cell.get_name(), "Actor stopped");
            }
            SupervisionEvent::ActorFailed(actor_cell, error) => {
                error!("{error:?}");
                info!(actor = actor_cell.get_name(), "Restarting failed actor");
                if let Some(name) = actor_cell.get_name() {
                    match name.as_str() {
                        "spawner" => start_spawner(&myself, state.client.clone()).await?,
                        "config_provider" => {
                            start_config_provider(&myself, state.config_path.clone()).await?
                        }
                        "moderator" => start_moderator(&myself, state.client.clone()).await?,
                        _ => {}
                    }
                }
            }
            SupervisionEvent::ProcessGroupChanged(_) => {
                info!("PG changed");
            }
        };
        Ok(())
    }
}

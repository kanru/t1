use matrix_sdk::Client;
use ractor::{Actor, ActorProcessingErr, ActorRef, SupervisionEvent};

use super::{moderator::Moderator, spawner::Spawner};

pub(crate) struct Supervisor;

pub(crate) enum SupervisorMessage {}

async fn start_spawner(myself: &ActorRef<SupervisorMessage>) -> anyhow::Result<()> {
    Actor::spawn_linked(Some("spawner".into()), Spawner, (), myself.get_cell()).await?;
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
    type State = Client;
    type Arguments = Client;

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        start_spawner(&myself).await?;
        start_moderator(&myself, args.clone()).await?;

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
                tracing::info!(actor = actor_cell.get_name(), "Actor started");
            }
            SupervisionEvent::ActorTerminated(actor_cell, _, _) => {
                tracing::info!(actor = actor_cell.get_name(), "Actor stopped");
            }
            SupervisionEvent::ActorFailed(actor_cell, error) => {
                tracing::error!(
                    actor = actor_cell.get_name(),
                    error = error,
                    "Restarting failed actor"
                );
                if let Some(name) = actor_cell.get_name() {
                    match name.as_str() {
                        "spawner" => start_spawner(&myself).await?,
                        "moderator" => start_moderator(&myself, state.clone()).await?,
                        _ => {}
                    }
                }
            }
            SupervisionEvent::ProcessGroupChanged(_) => {
                tracing::info!("PG changed");
            }
        };
        Ok(())
    }
}

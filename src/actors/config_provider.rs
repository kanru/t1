use std::{fs, path::PathBuf};

use ractor::{Actor, RpcReplyPort};

use crate::config::T1Config;

pub(crate) struct ConfigProvider;

pub(crate) enum ConfigProviderMessage {
    GetConfig(RpcReplyPort<T1Config>),
}

impl Actor for ConfigProvider {
    type Msg = ConfigProviderMessage;
    type State = PathBuf;
    type Arguments = PathBuf;

    async fn pre_start(
        &self,
        _myself: ractor::ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ractor::ActorProcessingErr> {
        Ok(args)
    }

    async fn handle(
        &self,
        _myself: ractor::ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ractor::ActorProcessingErr> {
        match message {
            ConfigProviderMessage::GetConfig(reply) => {
                let config_text = fs::read_to_string(state)?;
                let config: T1Config = toml::from_str(&config_text)?;
                reply.send(config)?;
            }
        };
        Ok(())
    }
}

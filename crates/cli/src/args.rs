use serde::Deserialize;
use serde::Serialize;

#[derive(clap::ArgEnum, Clone, Debug, Default, Serialize, Deserialize)]
pub enum HostMode {
    /// Trivial Wasm Host Evnironment
    #[default]
    DEFAULT,

    /// Wasm Host Envionment with more Zk plugins
    STANDARD,
}

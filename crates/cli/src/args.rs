use serde::Deserialize;
use serde::Serialize;

#[derive(clap::ArgEnum, Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub enum HostMode {
    /// Trivial Wasm Host Environment
    #[default]
    DEFAULT,

    /// Wasm Host Environment with more Zk plugins
    STANDARD,
}

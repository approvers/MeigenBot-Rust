pub mod command;
pub mod db;
pub mod entrypoint;
pub mod model;
pub mod util;

pub type Synced<T> = std::sync::Arc<tokio::sync::RwLock<T>>;

#[cfg(feature = "memorydb")]
pub mod mem;
#[cfg(feature = "mongodb_")]
pub mod mongo;

use anyhow::Result;
use async_trait::async_trait;

use crate::model::Meigen;

#[derive(Default)]
pub struct FindOptions<'a> {
    pub author: Option<&'a str>,
    pub content: Option<&'a str>,
    pub offset: u32,
    pub limit: u8,
}

#[async_trait]
pub trait MeigenDatabase: Send + Sync + 'static {
    async fn save(&mut self, author: String, content: String) -> Result<Meigen>;
    async fn load(&self, id: u32) -> Result<Option<Meigen>>;
    async fn load_bulk(&self, id: &[u32]) -> Result<Vec<Meigen>>;
    async fn delete(&mut self, id: u32) -> Result<bool>;

    async fn get_current_id(&self) -> Result<u32>;

    async fn find(&self, options: FindOptions<'_>) -> Result<Vec<Meigen>>;

    async fn count(&self) -> Result<u32>;

    async fn append_loved_user(&mut self, id: u32, loved_user_id: u64) -> Result<bool>;
    async fn remove_loved_user(&mut self, id: u32, loved_user_id: u64) -> Result<bool>;
}

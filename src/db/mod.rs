pub mod mem;

use {crate::model::Meigen, anyhow::Result, async_trait::async_trait};

#[derive(Default)]
pub struct FindOptions<'a> {
    pub author: Option<&'a str>,
    pub content: Option<&'a str>,
    pub offset: u64,
    pub limit: u8,
}

#[async_trait]
pub trait MeigenDatabase: Send + Sync {
    async fn save(&mut self, meigen: &Meigen) -> Result<()>;
    async fn load(&self, id: u64) -> Result<Option<Meigen>>;
    async fn delete(&mut self, id: u64) -> Result<()>;

    async fn get_current_id(&self) -> Result<u64>;

    async fn find(&self, options: FindOptions<'_>) -> Result<Vec<Meigen>>;

    async fn count(&self) -> Result<u64>;
}

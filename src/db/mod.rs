use {
    crate::model::{Meigen, MeigenID},
    anyhow::Result,
    async_trait::async_trait,
};

#[derive(Default)]
pub struct FindOptions<'a> {
    pub author: Option<&'a str>,
    pub content: Option<&'a str>,
    pub offset: u64,
    pub limit: u8,
}

#[async_trait]
pub trait MeigenDatabase {
    fn save(&mut self, meigen: &Meigen) -> Result<()>;
    fn load(&self, id: MeigenID) -> Result<Option<Meigen>>;
    fn delete(&mut self, id: MeigenID) -> Result<()>;

    fn get_current_id(&self) -> Result<u64>;

    fn find(&self, options: FindOptions<'_>) -> Result<Vec<Option<Meigen>>>;
}

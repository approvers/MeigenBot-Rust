pub mod mem;
pub mod mongo;

use {crate::model::Meigen, anyhow::Result, async_trait::async_trait};

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
    async fn delete(&mut self, id: u32) -> Result<bool>;

    async fn get_current_id(&self) -> Result<u32>;

    async fn find(&self, options: FindOptions<'_>) -> Result<Vec<Meigen>>;

    async fn count(&self) -> Result<u32>;
}

trait IteratorEditExt<T> {
    fn edit<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut T);
}

impl<T, E> IteratorEditExt<T> for std::result::Result<T, E> {
    fn edit<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut T),
    {
        match self {
            Ok(ref mut t) => f(t),
            Err(_) => {}
        };

        self
    }
}

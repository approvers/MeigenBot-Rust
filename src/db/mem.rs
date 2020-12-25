use {
    crate::{
        db::{FindOptions, MeigenDatabase},
        model::Meigen,
    },
    anyhow::{anyhow, Result},
    async_trait::async_trait,
};

pub struct MemoryMeigenDatabase {
    inner: Vec<Meigen>,
}

#[async_trait]
impl MeigenDatabase for MemoryMeigenDatabase {
    async fn get_current_id(&self) -> Result<u64> {
        Ok(self.inner.iter().map(|x| x.id).max().unwrap_or(0))
    }

    async fn save(&mut self, meigen: &Meigen) -> Result<()> {
        self.inner.push(meigen.clone());
        Ok(())
    }

    async fn load(&self, id: u64) -> Result<Option<Meigen>> {
        Ok(self.inner.iter().find(|x| x.id == id).cloned())
    }

    async fn delete(&mut self, id: u64) -> Result<()> {
        let pos = self
            .inner
            .iter()
            .position(|x| x.id == id)
            .ok_or_else(|| anyhow!("couldn't find meigen which has specified ID"))?;

        self.inner.remove(pos);

        Ok(())
    }

    async fn find(&self, options: FindOptions<'_>) -> Result<Vec<Meigen>> {
        Ok(self
            .inner
            .iter()
            .flat_map(|x| {
                if let Some(author) = options.author {
                    if !x.author.contains(author) {
                        return None;
                    }
                }

                if let Some(content) = options.content {
                    if !x.content.contains(content) {
                        return None;
                    }
                }

                Some(x)
            })
            .skip(options.offset as _)
            .take(options.limit as _)
            .cloned()
            .collect())
    }

    async fn count(&self) -> Result<u64> {
        Ok(self.inner.len() as _)
    }
}

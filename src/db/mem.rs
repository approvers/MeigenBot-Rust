use anyhow::Result;
use async_trait::async_trait;

use crate::{
    db::{FindOptions, MeigenDatabase},
    model::Meigen,
};

#[derive(Default)]
pub struct MemoryMeigenDatabase {
    inner: Vec<Meigen>,
}

impl MemoryMeigenDatabase {
    pub fn new() -> Self {
        Self { inner: vec![] }
    }
}

#[async_trait]
impl MeigenDatabase for MemoryMeigenDatabase {
    async fn get_current_id(&self) -> Result<u32> {
        Ok(self.inner.iter().map(|x| x.id).max().unwrap_or(0))
    }

    async fn save(&mut self, author: String, content: String) -> Result<Meigen> {
        let id = self.get_current_id().await?;

        let meigen = Meigen {
            id: id + 1,
            author,
            content,
            loved_user_id: Vec::new()
        };

        self.inner.push(meigen.clone());

        Ok(meigen)
    }

    async fn load(&self, id: u32) -> Result<Option<Meigen>> {
        Ok(self.inner.iter().find(|x| x.id == id).cloned())
    }

    async fn load_bulk(&self, id: &[u32]) -> Result<Vec<Meigen>> {
        Ok(self
            .inner
            .iter()
            .filter(|x| id.iter().any(|&y| y == x.id))
            .cloned()
            .collect())
    }

    async fn delete(&mut self, id: u32) -> Result<bool> {
        let pos = self.inner.iter().position(|x| x.id == id);

        Ok(match pos {
            Some(pos) => {
                self.inner.remove(pos);
                true
            }

            None => false,
        })
    }

    async fn find(&self, options: FindOptions<'_>) -> Result<Vec<Meigen>> {
        Ok(self
            .inner
            .iter()
            .rev()
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

    async fn count(&self) -> Result<u32> {
        Ok(self.inner.len() as _)
    }

    async fn append_loved_user(&mut self, id: u32, loved_user_id: u64) -> Result<bool> {
        let mut meigen = self.inner.iter_mut()
            .find(|x| x.id == id);

        if meigen.is_none() {
            return Ok(false);
        }
        let mut meigen = meigen.unwrap();

        if meigen.is_loving(loved_user_id) {
            return Ok(false);
        }

        meigen.loved_user_id.push(loved_user_id);

        Ok(true)
    }

    async fn remove_loved_user(&mut self, id: u32, loved_user_id: u64) -> Result<bool> {
        let mut meigen = self.inner.iter_mut()
            .find(|x| x.id == id);

        if meigen.is_none() {
            return Ok(false)
        }
        let mut meigen = meigen.unwrap();

        let pos = meigen.loved_user_id.iter()
            .position(|id| id == loved_user_id);

        match pos {
            Ok(p) => {
                meigen.loved_user_id.remove(p);
                Ok(true)
            },
            None => Ok(false)
        }
    }
}

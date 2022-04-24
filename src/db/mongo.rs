use std::convert::{TryFrom, TryInto};
use anyhow::{Context, Result};
use async_trait::async_trait;
use mongodb::{
    bson::{doc, from_document, Document},
    options::ClientOptions,
    Client, Collection,
};
use serde::{Deserialize, Serialize};
use tokio_stream::StreamExt;

use super::FindOptions;
use crate::{db::MeigenDatabase, model::Meigen, util::IteratorEditExt};

#[derive(Serialize, Deserialize, Clone)]
struct MongoMeigen {
    id: i64,
    author: String,
    content: String,

    // Added in PR #17. The attribute is for the backward compatibility.
    #[serde(default)]
    loved_user_id: Vec<String>
}

impl TryFrom<MongoMeigen> for Meigen {
    type Error = anyhow::Error;

    fn try_from(m: MongoMeigen) -> Result<Meigen> {
        let loved_user_id = m.loved_user_id.iter()
            .map(
                |id| id
                    .parse()
                    .context("DB contains invalid value")
            )
            .collect::<Result<Vec<u64>>>()?;

        Ok(Meigen {
            id: m.id as _,
            author: m.author,
            content: m.content,
            loved_user_id
        })
    }
}

pub struct MongoMeigenDatabase {
    inner: Collection<MongoMeigen>,
}

impl MongoMeigenDatabase {
    pub async fn new(url: &str) -> Result<Self> {
        let opt = ClientOptions::parse(url)
            .await
            .context("failed to parse mongodb url")?;

        let collection = Client::with_options(opt)
            .context("failed to create mongodb client")?
            .database("meigen")
            .collection("entries");

        Ok(Self { inner: collection })
    }
}

#[async_trait]
impl MeigenDatabase for MongoMeigenDatabase {
    async fn save(&mut self, author: String, content: String) -> anyhow::Result<Meigen> {
        // FIXME: use transaction
        let current_id = self
            .get_current_id()
            .await
            .context("failed to get current head meigen id")? as i64;

        let id = current_id + 1;

        let meigen = MongoMeigen {
            id,
            author,
            content,
            loved_user_id: Vec::new()
        };

        self.inner
            .insert_one(meigen.clone(), None)
            .await
            .context("failed to insert meigen")?;

        meigen.try_into()
    }

    async fn load(&self, id: u32) -> anyhow::Result<Option<Meigen>> {
        self.inner
            .find_one(doc! { "id": id }, None)
            .await
            .context("failed to find meigen")?
            .map(TryFrom::try_from)
            .transpose()
            .context("failed to deserialize the meigen")
    }

    async fn load_bulk(&self, id: &[u32]) -> anyhow::Result<Vec<Meigen>> {
        self.inner
            .find(doc! { "id": { "$in": id } }, None)
            .await
            .context("failed to make find request")?
            .map(
                |x| x
                    .map(TryFrom::try_from)
                    .context("failed to deserialize the meigen")?
            )
            .collect::<Result<Vec<_>, _>>()
            .await
            .context("failed to decode meigen")
    }

    async fn delete(&mut self, id: u32) -> anyhow::Result<bool> {
        self.inner
            .delete_one(doc! { "id": id }, None)
            .await
            .context("failed to delete meigen")
            .map(|x| x.deleted_count == 1)
    }

    async fn get_current_id(&self) -> anyhow::Result<u32> {
        self.inner
            .aggregate(
                vec![doc! {
                    "$group": {
                        "_id": "",
                        "current_id": {
                            "$max": "$id"
                        }
                    }
                }],
                None,
            )
            .await
            .context("failed to aggregate")?
            .next()
            .await
            .context("aggregation returned nothing")?
            .context("failed to fetch aggregated result")?
            .get("current_id")
            .context("returned document doesn't have current_id property")?
            .as_i64()
            .context("returned document's current_id property isn't i64")
            .map(|x| x as u32)
    }

    async fn find(&self, options: FindOptions<'_>) -> anyhow::Result<Vec<Meigen>> {
        self.inner
            .aggregate(
                vec![
                    {
                        let into_regex = |x| doc! { "$regex": format!(".*{}.*", regex::escape(x)) };
                        let mut doc = Document::new();

                        if let Some(author) = options.author {
                            doc.insert("author", into_regex(author));
                        }

                        if let Some(content) = options.content {
                            doc.insert("content", into_regex(content));
                        }

                        doc! { "$match": doc }
                    },
                    doc! { "$sort": { "id": -1 } },
                    doc! { "$skip": options.offset },
                    doc! { "$limit": options.limit as u32 },
                ],
                None,
            )
            .await
            .context("failed to aggregate")?
            .map(|x| x.context("failed to decode document"))
            .map(|x| {
                x.and_then(|x| {
                    from_document::<MongoMeigen>(x).context("failed to deserialize document")
                })
            })
            .map(
                |x| x
                    .map(TryFrom::try_from)
                    .context("failed to deserialize the meigen")?
            )
            .collect::<Result<Vec<Meigen>, _>>()
            .await
            .edit(|x| x.sort_unstable_by_key(|x| x.id))
            .context("failed to fetch aggregated documents")
    }

    async fn count(&self) -> anyhow::Result<u32> {
        self.inner
            .aggregate(vec![doc! { "$count": "id" }], None)
            .await
            .context("failed to aggregate")?
            .next()
            .await
            .context("aggregation returned nothing")?
            .context("failed to fetch aggregated result")?
            .get("id")
            .context("returned document doesn't have id property")?
            .as_i32()
            .context("returned document's id property wasn't i64")
            .map(|x| x as u32)
    }

    async fn append_loved_user(&mut self, id: u32, loved_user_id: u64) -> Result<bool> {
        self.inner
            .update_one(
                doc! { "id": id },
                doc! { "$addToSet": { "loved_user_id": loved_user_id.to_string() } },
                None
            )
            .await
            .context("failed to append loved user id")
            .map(|x| x.modified_count == 1)
    }

    async fn remove_loved_user(&mut self, id: u32, loved_user_id: u64) -> Result<bool> {
        self.inner
            .update_one(
                doc! { "id": id },
                doc! { "$pull": { "loved_user_id": loved_user_id.to_string() } },
                None
            )
            .await
            .context("failed to remove loved user id")
            .map(|x| x.modified_count == 1)
    }
}

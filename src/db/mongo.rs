use {
    super::{FindOptions, IteratorEditExt},
    crate::{db::MeigenDatabase, model::Meigen},
    anyhow::{Context, Result},
    async_trait::async_trait,
    mongodb::{
        bson::{doc, from_document, to_document, Document},
        options::ClientOptions,
        Client, Collection,
    },
    serde::{Deserialize, Serialize},
    tokio::stream::StreamExt,
};

#[derive(Serialize, Deserialize)]
struct MongoMeigen {
    id: i64,
    author: String,
    content: String,
}

pub struct MongoMeigenDatabase {
    inner: Collection,
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
        let current_id = self
            .get_current_id()
            .await
            .context("failed to get current head meigen id")?;

        let meigen = Meigen {
            id: current_id + 1,
            author,
            content,
        };

        let doc = to_document(&meigen).context("failed to serialize meigen to bson document")?;

        self.inner
            .insert_one(doc, None)
            .await
            .context("failed to insert meigen")?;

        Ok(meigen)
    }

    async fn load(&self, id: u32) -> anyhow::Result<Option<Meigen>> {
        self.inner
            .find_one(doc! { "id": id }, None)
            .await
            .context("failed to fetch meigen")?
            .map(from_document)
            .transpose()
            .context("failed to deserialize meigen")
    }

    async fn delete(&mut self, id: u32) -> anyhow::Result<bool> {
        self.inner
            .delete_one(doc! { "id": id }, None)
            .await
            .context("failed to delete meigen")
            .map(|x| x.deleted_count == 0)
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
                    doc! { "$skip": options.offset },
                    doc! { "$limit": options.limit as u32 },
                ],
                None,
            )
            .await
            .context("failed to aggregate")?
            .collect::<Result<Vec<_>, _>>()
            .await
            .context("failed to fetch aggregated documents")?
            .into_iter()
            .map(from_document)
            .collect::<Result<Vec<Meigen>, _>>()
            .edit(|x| x.sort_by_key(|x| x.id))
            .context("failed to deserialize result")
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
}

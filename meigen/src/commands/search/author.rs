use super::listify;
use crate::db::MeigenDatabase;
use crate::{CommandResult, Error};
use std::sync::Arc;
use tokio::sync::RwLock;

pub(crate) async fn author<D>(
    db: &Arc<RwLock<D>>,
    target_author: &str,
    show_count: i32,
    page_num: i32,
) -> CommandResult
where
    D: MeigenDatabase,
{
    let meigens = db
        .read()
        .await
        .search_by_author(target_author)
        .await
        .map_err(|x| Error::DatabaseError(Box::new(x)))?;

    listify(&meigens, show_count, page_num)
}

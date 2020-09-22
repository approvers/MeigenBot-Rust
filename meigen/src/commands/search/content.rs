use super::listify;
use crate::db::MeigenDatabase;
use crate::{CommandResult, Error};
use std::sync::Arc;
use tokio::sync::RwLock;

pub(crate) async fn content<D>(
    db: &Arc<RwLock<D>>,
    target_content: &str,
    show_count: i32,
    page_num: i32,
) -> CommandResult
where
    D: MeigenDatabase,
{
    let meigens = db
        .read()
        .await
        .search_by_content(target_content)
        .await
        .map_err(|x| Error::DatabaseError(Box::new(x)))?;

    listify(&meigens, show_count, page_num)
}

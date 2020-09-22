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
) -> CommandResult<D>
where
    D: MeigenDatabase,
{
    let meigens = db
        .read()
        .await
        .search_by_author(target_author)
        .await
        .map_err(Error::DatabaseError)?;

    listify::<D>(&meigens, show_count, page_num)
}

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
) -> CommandResult<D>
where
    D: MeigenDatabase,
{
    let meigens = db
        .read()
        .await
        .search_by_content(target_content)
        .await
        .map_err(Error::DatabaseError)?;

    listify::<D>(&meigens, show_count, page_num)
}

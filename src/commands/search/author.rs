use crate::commands::listify;
use crate::commands::Error;
use crate::commands::Result;
use crate::db::MeigenDatabase;
use std::sync::Arc;
use std::sync::RwLock;

pub async fn author(
    db: &Arc<RwLock<impl MeigenDatabase>>,
    target_author: &str,
    show_count: i32,
    page_num: i32,
) -> Result {
    let meigens = db
        .read()
        .unwrap()
        .search_by_author(target_author)
        .await
        .map_err(Error::load_failed)?;

    listify(&meigens, show_count, page_num)
}

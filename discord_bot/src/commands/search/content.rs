use super::listify;
use crate::commands::{Error, Result};
use crate::db::MeigenDatabase;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn content(
    db: &Arc<RwLock<impl MeigenDatabase>>,
    target_content: &str,
    show_count: i32,
    page_num: i32,
) -> Result {
    let meigens = db
        .read()
        .await
        .search_by_content(target_content)
        .await
        .map_err(Error::load_failed)?;

    listify(&meigens, show_count, page_num)
}

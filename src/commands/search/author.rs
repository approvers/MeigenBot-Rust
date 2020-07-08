use crate::commands::listify;
use crate::commands::Error;
use crate::commands::Result;
use crate::db::MeigenDatabase;

pub async fn author(
    db: &impl MeigenDatabase,
    target_author: &str,
    show_count: i32,
    page_num: i32,
) -> Result {
    let meigens = db.meigens().await.map_err(Error::load_failed)?;

    let filtered = meigens
        .iter()
        .filter(|x| x.author.contains(target_author))
        .collect::<Vec<&_>>();

    listify(filtered.as_slice(), show_count, page_num)
}

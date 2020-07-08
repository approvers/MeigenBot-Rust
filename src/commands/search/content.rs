use crate::commands::listify;
use crate::commands::Error;
use crate::commands::Result;
use crate::db::MeigenDatabase;

pub async fn content(
    db: &impl MeigenDatabase,
    target_content: &str,
    show_count: i32,
    page_num: i32,
) -> Result {
    let meigens = db.meigens().await.map_err(Error::load_failed)?;

    let filtered = meigens
        .iter()
        .filter(|x| x.content.contains(target_content))
        .collect::<Vec<&_>>();

    listify(filtered.as_slice(), show_count, page_num)
}

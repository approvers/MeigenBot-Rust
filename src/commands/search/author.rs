use crate::commands::listify;
use crate::commands::Result;
use crate::db::MeigenDatabase;
use crate::db::RegisteredMeigen;

pub async fn author(
    db: &impl MeigenDatabase,
    target_author: &str,
    show_count: i32,
    page_num: i32,
) -> Result {
    let filtered = db
        .meigens()
        .await
        .iter()
        .filter(|x| x.author.contains(target_author))
        .collect::<Vec<&RegisteredMeigen>>();

    listify(filtered.as_slice(), show_count, page_num)
}

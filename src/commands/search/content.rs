use crate::commands::listify;
use crate::commands::Result;
use crate::db::MeigenDatabase;
use crate::db::RegisteredMeigen;

pub async fn content(
    db: &impl MeigenDatabase,
    target_content: &str,
    show_count: i32,
    page_num: i32,
) -> Result {
    let filtered = db
        .meigens()
        .await
        .iter()
        .filter(|x| x.content.contains(target_content))
        .collect::<Vec<&RegisteredMeigen>>();

    listify(filtered.as_slice(), show_count, page_num)
}

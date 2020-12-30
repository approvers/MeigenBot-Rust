use {
    crate::{
        db::{FindOptions, MeigenDatabase},
        util::IteratorEditExt,
        Synced,
    },
    anyhow::Result,
};

pub async fn search_content(
    db: Synced<impl MeigenDatabase>,
    content: &str,
    show_count: Option<u8>,
    page: Option<u32>,
) -> Result<String> {
    let page = page.unwrap_or(0);
    let (show_count, clamp_msg) = option!({
        value: show_count,
        default: 5,
        min: 1,
        max: 10
    });

    use super::super::find;
    find(
        db,
        FindOptions {
            author: None,
            content: Some(content),
            offset: page * (show_count as u32),
            limit: show_count,
        },
    )
    .await
    .edit(|x| x.insert_str(0, clamp_msg))
}

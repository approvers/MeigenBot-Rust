use {
    crate::{db::MeigenDatabase, Synced},
    anyhow::{Context, Result},
};

pub async fn id(db: Synced<impl MeigenDatabase>, id: u32) -> Result<String> {
    let meigen = db
        .read()
        .await
        .load(id)
        .await
        .context("failed to get meigen")?;

    Ok(match meigen {
        Some(m) => format!("{}", m),
        None => "そのIDを持つ名言はありません".into(),
    })
}

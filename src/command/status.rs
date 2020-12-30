use {
    crate::{db::MeigenDatabase, Synced},
    anyhow::{Context, Result},
};

pub async fn status(db: Synced<impl MeigenDatabase>) -> Result<String> {
    let count = db
        .read()
        .await
        .count()
        .await
        .context("Failed to fetch meigen count")?;

    Ok(format!(
        "```yaml
total_count: {}
```",
        count
    ))
}

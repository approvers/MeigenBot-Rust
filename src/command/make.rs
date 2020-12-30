use {
    crate::{db::MeigenDatabase, Synced},
    anyhow::Result,
};

const MEIGEN_LENGTH_LIMIT: usize = 300;

pub async fn make(db: Synced<impl MeigenDatabase>, author: &str, content: &str) -> Result<String> {
    let strip = |s: &str| s.replace("`", "");

    let author = strip(author);
    let content = strip(content);

    if author.chars().count() + content.chars().count() > MEIGEN_LENGTH_LIMIT {
        return Ok("名言が長すぎます。もっと短くしてください。".into());
    }

    let meigen = db.write().await.save(author, content).await?;

    Ok(format!("{}", meigen))
}

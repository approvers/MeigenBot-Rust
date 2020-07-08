use crate::commands::Error;
use crate::commands::Result;
use crate::db::MeigenDatabase;

pub async fn status(db: &impl MeigenDatabase) -> Result {
    let meigens = db.meigens().await.map_err(Error::load_failed)?;
    let text = format!("合計名言数: {}個", meigens.len());

    Ok(text)
}

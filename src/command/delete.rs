use {
    crate::{db::MeigenDatabase, Synced},
    anyhow::{Context, Result},
};

const KAWAEMON_DISCORD_USER_ID: u64 = 391857452360007680;

pub async fn delete(
    db: Synced<impl MeigenDatabase>,
    meigen_id: u32,
    user_id: u64,
) -> Result<String> {
    if user_id != KAWAEMON_DISCORD_USER_ID {
        return Ok("このコマンドはかわえもんにしか実行できません".into());
    }

    let deleted = db
        .write()
        .await
        .delete(meigen_id)
        .await
        .context("failed to delete meigen")?;

    Ok(if deleted {
        "削除しました"
    } else {
        "そのIDを持つ名言は存在しません"
    }
    .into())
}

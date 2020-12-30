use {
    crate::{
        db::{FindOptions, MeigenDatabase},
        util::IteratorEditExt,
        Synced,
    },
    anyhow::{Context, Result},
};

const LIST_LENGTH_LIMIT: usize = 400;

trait IterExt {
    fn fold_list(self) -> String;
}

impl<T, D> IterExt for T
where
    D: std::fmt::Display,
    T: Iterator<Item = D> + DoubleEndedIterator,
{
    fn fold_list(self) -> String {
        let (mut text, len) = self
            .rev()
            .fold((String::new(), 0), |(mut text, mut len), meigen| {
                if len < LIST_LENGTH_LIMIT {
                    let meigen = format!("{}\n", meigen);

                    text.insert_str(0, &meigen);
                    len += meigen.chars().count() + 1;
                }

                (text, len)
            });

        if len >= LIST_LENGTH_LIMIT {
            text.insert_str(0, "結果が長すぎたため、一部の名言は省略されました。\n");
        }

        text
    }
}

/// clamps number, returns clamped number and message which is sent to User
macro_rules! option {
    ({value: $value:ident, default: $default:literal, min: $min:literal, max: $max:literal $(,)?}) => {{
        match $value.unwrap_or($default) {
            n if n > $max => (
                $max,
                concat!(
                    stringify!($value),
                    "の値は大きすぎたため",
                    stringify!($max),
                    "に丸められました。\n"
                ),
            ),

            n if n < $min => (
                $max,
                concat!(
                    stringify!($value),
                    "の値は小さすぎたため",
                    stringify!($max),
                    "に丸められました。\n"
                ),
            ),

            n => (n, ""),
        }
    }};
}

mod help;
pub use help::help;

mod status;
pub use status::status;

mod random;
pub use random::random;

mod make;
pub use make::make;

async fn find(db: Synced<impl MeigenDatabase>, opt: FindOptions<'_>) -> Result<String> {
    Ok(db.read().await.find(opt).await?.into_iter().fold_list())
}

mod search;
pub use search::author::search_author;

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

pub async fn list(
    db: Synced<impl MeigenDatabase>,
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

    find(
        db,
        FindOptions {
            author: None,
            content: None,
            offset: (show_count as u32) * page,
            limit: show_count,
        },
    )
    .await
    .edit(|x| x.insert_str(0, clamp_msg))
}

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

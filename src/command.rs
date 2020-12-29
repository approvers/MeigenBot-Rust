use {
    crate::{
        db::{FindOptions, MeigenDatabase},
        model::Meigen,
        util::IteratorEditExt,
        Synced,
    },
    anyhow::{Context, Result},
    rand::{rngs::StdRng, Rng, SeedableRng},
    std::{future::Future, pin::Pin},
};

const MEIGEN_LENGTH_LIMIT: usize = 300;
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

pub async fn help() -> Result<String> {
    const HELP_TEXT: &str = "```asciidoc
= meigen-bot-rust =
g!meigen [subcommand] [args...]
= subcommands =
    help                                    :: この文を出します
    make [作者] [名言]                       :: 名言を登録します
    list [表示数=5] [ページ=1]                :: 名言をリスト表示します
    id [名言ID]                             :: 指定されたIDの名言を表示します
    search [検索内容] [表示数=5] [ページ=1]    :: 名言を検索します(g!meigen searchでヘルプを表示します)
    random [表示数=1]                       :: ランダムに名言を出します
    status                                 :: 現在登録されてる名言の数を出します
    delete [名言ID]                         :: 指定されたIDの名言を削除します かわえもんにしか使えません
```";

    Ok(HELP_TEXT.into())
}

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

pub async fn random(db: Synced<impl MeigenDatabase>, count: Option<u8>) -> Result<String> {
    let (count, clamp_msg) = option!({
        value: count,
        default: 1,
        min: 1,
        max: 5,
    });

    fn get_random<'a>(
        db: &'a Synced<impl MeigenDatabase>,
        max: u32,
    ) -> Pin<Box<dyn Future<Output = Result<Meigen>> + Send + 'a>> {
        Box::pin(async move {
            let pos = StdRng::from_rng(&mut rand::thread_rng())
                .unwrap()
                .gen_range(1..=max);

            match db.read().await.load(pos).await? {
                Some(e) => Ok(e),
                None => get_random(db, max).await,
            }
        })
    }

    let mut meigens = Vec::with_capacity(count as _);
    let max = db.read().await.get_current_id().await?;

    for _ in 0..count {
        meigens.push(get_random(&db, max).await?);
    }

    meigens.sort_by_key(|x| x.id);

    let mut msg = meigens.into_iter().fold_list();
    msg.insert_str(0, clamp_msg);

    Ok(msg)
}

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

async fn find(db: Synced<impl MeigenDatabase>, opt: FindOptions<'_>) -> Result<String> {
    Ok(db.read().await.find(opt).await?.into_iter().fold_list())
}

pub async fn search_author(
    db: Synced<impl MeigenDatabase>,
    author: &str,
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
            author: Some(author),
            content: None,
            offset: page * (show_count as u32),
            limit: show_count,
        },
    )
    .await
    .edit(|x| x.insert_str(0, clamp_msg))
}

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

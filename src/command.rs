use anyhow::{anyhow, Context as _, Result};
use rand::{prelude::SmallRng, Rng, SeedableRng};

use crate::{
    db::{FindOptions, MeigenDatabase},
    model::Meigen,
    util::IteratorEditExt,
    Synced,
};

const MEIGEN_LENGTH_LIMIT: usize = 300;
const LIST_LENGTH_LIMIT: usize = 400;

trait IterExt {
    fn fold_list(self) -> Option<String>;
}

impl<T, D> IterExt for T
where
    D: std::fmt::Display,
    T: Iterator<Item = D> + DoubleEndedIterator,
{
    fn fold_list(self) -> Option<String> {
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

        match len {
            0 => None,
            _ => Some(text),
        }
    }
}

/// clamps number, returns clamped number and message which should be sent to User
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

    let count = count as usize;
    let max = db.read().await.get_current_id().await?;

    if count > max as usize {
        return Ok("countが総名言数を超えています。".into());
    }

    let mut rng = SmallRng::from_rng(&mut rand::thread_rng()).unwrap();

    let mut meigens = Vec::<Meigen>::with_capacity(count);

    while meigens.len() != count {
        let want = meigens.len() - count;
        let mut try_fetch = Vec::with_capacity(want);

        loop {
            let new_id_candidate = rng.gen_range(1..=max);

            if try_fetch.contains(&new_id_candidate) {
                continue;
            }

            if meigens.iter().any(|x| x.id == new_id_candidate) {
                continue;
            }

            try_fetch.push(new_id_candidate);
            if try_fetch.len() == want {
                break;
            }
        }

        let mut fetched = db.read().await.load_bulk(&try_fetch).await?;
        meigens.append(&mut fetched);
    }

    let mut msg = meigens
        .into_iter()
        .fold_list()
        .ok_or_else(|| anyhow!("random::get_random didn't bring any meigen"))?;

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

async fn find(db: Synced<impl MeigenDatabase>, opt: FindOptions<'_>) -> Result<Option<String>> {
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
    .edit(|x: &mut String| x.insert_str(0, clamp_msg))
    .map(|x| x.unwrap_or_else(|| "その条件に合致する名言は見つかりませんでした。".into()))
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
    .edit(|x: &mut String| x.insert_str(0, clamp_msg))
    .map(|x| x.unwrap_or_else(|| "その条件に合致する名言はみつかりませんでした。".into()))
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
    .edit(|x: &mut String| x.insert_str(0, clamp_msg))
    .map(|x| x.unwrap_or_else(|| "その条件に合致する名言はみつかりませんでした。".into()))
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

pub async fn gophersay(db: Synced<impl MeigenDatabase>, id: u32) -> Result<String> {
    let meigen = db
        .read()
        .await
        .load(id)
        .await
        .context("failed to get meigen")?;

    let meigen = match meigen {
        None => return Ok("そのIDを持つ名言はありません".into()),

        Some(meigen) => format!(
            "{}
    --- {}",
            meigen.content, meigen.author
        )
        .lines()
        .collect::<Vec<_>>()
        .join("\n  "),
    };

    let bar_length = meigen
        .lines()
        .map(|x| {
            x.chars()
                .map(|x| if x.is_ascii() { 1 } else { 2 })
                .sum::<usize>()
        })
        .max()
        .unwrap_or(30)
        + 4;

    let bar = "-".chars().cycle().take(bar_length).collect::<String>();

    let msg = format!(
        "```
{}
  {}
{}
{}
```",
        bar,
        meigen,
        bar,
        include_str!("./gopher.ascii")
    );

    Ok(msg)
}

pub async fn love(db: Synced<impl MeigenDatabase>, id: u32, from_user_id: u64) -> Result<String> {
    let updated = db
        .write()
        .await
        .append_loved_user(id, from_user_id)
        .await
        .context("failed to append the loved user id")?;

    Ok(if updated {
        "いいねをしました。"
    } else {
        "いいねできませんでした。名言がないか、既にいいねをしています。"
    }.into())
}

pub async fn unlove(db: Synced<impl MeigenDatabase>, id: u32, from_user_id: u64) -> Result<String> {
    let updated = db
        .write()
        .await
        .remove_loved_user(id, from_user_id)
        .await
        .context("failed to append the loved user id")?;

    Ok(if updated {
        "いいねを取り消しました。"
    } else {
        "いいねを取り消しできませんでした。名言がないか、もともといいねをしていませんでした。"
    }.into())
}

pub(crate) mod delete;
pub(crate) mod export;
pub(crate) mod help;
pub(crate) mod id;
pub(crate) mod list;
pub(crate) mod make;
pub(crate) mod random;
pub(crate) mod search;
pub(crate) mod status;

pub(crate) use delete::delete;
pub(crate) use export::export;
pub(crate) use help::help;
pub(crate) use id::id;
pub(crate) use list::list;
pub(crate) use make::make;
pub(crate) use random::random;
pub(crate) use search::search;
pub(crate) use status::status;

// util
use crate::db::RegisteredMeigen;

fn internal_format(id: u32, author: &str, content: &str) -> String {
    format!(
        "Meigen No.{}\n```\n{}\n    --- {}\n```",
        id, content, author
    )
}

pub(crate) fn meigen_format(meigen: &RegisteredMeigen) -> String {
    internal_format(meigen.id, &meigen.author, &meigen.content)
}

fn meigen_tidy_format(meigen: &RegisteredMeigen, max_length: usize) -> String {
    const BASE: &str = "Meigen No.\n```\n\n    --- \n```";
    const TIDY_SUFFIX: &str = "...";
    const NO_SPACE_MSG: &str = "スペースが足りない...";

    let remain_length = (max_length as i32)
        - (BASE.chars().count() as i32)
        - (meigen.author.chars().count() as i32);

    // 十分なスペースがあるなら、そのままフォーマットして返す
    if remain_length >= meigen.content.chars().count() as i32 {
        return meigen_format(meigen);
    }

    // 作者名が長すぎるなどの理由で、...を使った省略でも入らない場合は、NO_SPACE_MSGを突っ込む
    if remain_length <= NO_SPACE_MSG.chars().count() as i32 {
        return format!("Meigen No.{}\n```{}```", meigen.id, NO_SPACE_MSG);
    }

    // 上記どれにも引っかからない場合、最後を...で削って文字列を減らして返す
    let content = meigen
        .content
        .chars()
        .take((remain_length - TIDY_SUFFIX.len() as i32) as usize)
        .chain(TIDY_SUFFIX.chars())
        .collect::<String>();

    internal_format(meigen.id, &meigen.author, &content)
}

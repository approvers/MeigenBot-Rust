mod delete;
mod help;
mod id;
mod list;
mod make;
mod random;
mod search;
mod status;

pub use delete::delete;
pub use help::help;
pub use id::id;
pub use list::list;
pub use make::make;
pub use random::random;
pub use search::search;
pub use status::status;

pub type Result = std::result::Result<String, Error>;

crate::make_error_enum! {
    Error;

    NotEnoughArgs not_enough_args() => "引数が足りないよ",

    TooLongMeigen too_long_meigen(actual_length, limit) => "{}文字は長過ぎません。。。？{}文字以下にしてください。。。",
    MeigenNotFound meigen_nf(id) => "ID{}を持つ名言は見つかりませんでした",
    NoMeigenMatches no_meigen_matches() => "一致する名言がなかったよ。。。",
    TooManyMatches too_many_meigen_matches() => "結果が長すぎて表示できないよ。もっと値を小さくしてね。",
    SaveFailed save_failed(e) => "名言保存に失敗しました: {}",

    ArgumentNumberParseFailed arg_num_parse_fail(th, e) => "{}番目の引数が正しい数値じゃないよ: {}",
    NumberParseFailed num_parse_fail(e) => "引数に正しくない数字が含まれているよ: {}",

    InvalidSearchSubCommand invalid_search_subcommand() => "検索コマンドが正しくないよ"
}

// util

use crate::db::RegisteredMeigen;

fn internal_format(id: u32, author: &str, content: &str) -> String {
    format!(
        "Meigen No.{}\n```\n{}\n    --- {}\n```",
        id, content, author
    )
}

pub fn meigen_format(meigen: &RegisteredMeigen) -> String {
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

fn listify(slice: &[&RegisteredMeigen], show_count: i32, page: i32) -> Result {
    const LIST_MAX_LENGTH: usize = 500;
    const MAX_LENGTH_PER_MEIGEN: usize = 50;

    let range = {
        use std::convert::TryInto;

        let meigens_end_index = slice.len() as i32;
        if meigens_end_index > show_count {
            let from: usize = {
                (meigens_end_index - show_count - (show_count * (page - 1)))
                    .try_into()
                    .map_err(Error::num_parse_fail)?
            };

            let to: usize = {
                (meigens_end_index - (show_count * (page - 1)))
                    .try_into()
                    .map_err(Error::num_parse_fail)?
            };

            &slice[from..to]
        } else {
            &slice[0..(slice.len())]
        }
    };

    let mut result = String::new();

    for meigen in range {
        let formatted = meigen_tidy_format(meigen, MAX_LENGTH_PER_MEIGEN);
        result += &format!("\n{}", &formatted);
    }

    if result.is_empty() {
        return Err(Error::no_meigen_matches());
    }

    if result.chars().count() >= LIST_MAX_LENGTH {
        return Err(Error::too_many_meigen_matches());
    }

    Ok(result)
}

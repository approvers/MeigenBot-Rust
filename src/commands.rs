mod author;
mod delete;
mod help;
mod id;
mod list;
mod make;
mod random;
mod status;

pub use author::author;
pub use delete::delete;
pub use help::help;
pub use id::id;
pub use list::list;
pub use make::make;
pub use random::random;
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
}

// util

use crate::db::RegisteredMeigen;

#[inline]
pub(self) fn listify(slice: &[&RegisteredMeigen], show_count: i32, page: i32) -> Result {
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

            from..to
        } else {
            0..(meigens_end_index as usize)
        }
    };

    let mut result = String::new();

    for index in range {
        let meigen = match slice.get(index) {
            Some(m) => m,
            None => break,
        };

        let formatted = meigen.tidy_format(MAX_LENGTH_PER_MEIGEN);
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

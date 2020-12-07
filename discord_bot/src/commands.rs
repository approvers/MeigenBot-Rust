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
    LoadFailed load_failed(e) => "名言の取得に失敗しました: {}",

    ArgumentNumberParseFailed arg_num_parse_fail(th, e) => "{}番目の引数が正しい数値じゃないよ: {}",
    NumberParseFailed num_parse_fail(e) => "引数に正しくない数字が含まれているよ: {}",

    InvalidSearchSubCommand invalid_search_subcommand() => "検索コマンドが正しくないよ",

    AdministratorOnlyCommand admin_only() => "そのコマンドはかわえもんにしか使えないよ"
}

// util

use db::RegisteredMeigen;

fn internal_format(id: u32, author: &str, content: &str) -> String {
    format!(
        "Meigen No.{}\n```\n{}\n    --- {}\n```",
        id, content, author
    )
}

pub fn meigen_format(meigen: &RegisteredMeigen) -> String {
    internal_format(meigen.id, &meigen.author, &meigen.content)
}

fn meigen_tidy_format(meigen: &RegisteredMeigen, _: usize) -> String {
    // 前までは名言がある程度長い場合末尾を省略するようにしていたのですが
    // どうやら余計な機能だったようなので削除しました。
    meigen_format(meigen)
}

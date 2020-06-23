use crate::commands::{Error, Result};
use crate::db::Database;
use crate::db::MeigenEntry;
use crate::message_parser::ParsedMessage;

use log::error;

pub fn make(db: &mut impl Database, message: ParsedMessage) -> Result {
    if message.args.len() <= 1 {
        return Err(Error::not_enough_args());
    }

    let author = message.args.get(0).unwrap().clone();
    let (content, checked_result) = {
        let author_skip_count = message.raw_args.find(&author).unwrap() + author.chars().count();
        let content = message
            .raw_args
            .trim()
            .chars()
            .skip(author_skip_count)
            .collect::<String>();

        strip_meigen(content.trim())
    };

    let new_meigen_entry = MeigenEntry::new(author, content).map_err(|err| {
        use crate::db::MeigenError;
        match err {
            MeigenError::TooLongMeigen { actual_size, limit } => {
                Error::too_long_meigen(actual_size, limit)
            }
        }
    })?;

    let registered_meigen = db.save_meigen(new_meigen_entry).map_err(|err| {
        error!("ファイル保存に失敗: {}", err);
        Error::save_failed(err)
    })?;

    let mut message = String::new();
    message += &checked_result.format();
    message += &registered_meigen.format();

    Ok(message)
}

pub struct CheckResult {
    replaced_back_quote: bool,
    reduced_code_block: bool,
}

// 名言に含まれている余分なものを取り除き、
// 取り除いた結果のStringと、何を取り除いたかを表すCheckResultを返す
pub fn strip_meigen(input: &str) -> (String, CheckResult) {
    const CODE_BLOCK: &str = "```";

    let mut result = input.to_string();
    let mut check_result = CheckResult {
        replaced_back_quote: false,
        reduced_code_block: false,
    };

    if result.starts_with(CODE_BLOCK) && result.ends_with(CODE_BLOCK) {
        check_result.reduced_code_block = true;

        let result_len = result.chars().count();
        result = result
            .chars()
            .take(result_len - CODE_BLOCK.len())
            .skip(CODE_BLOCK.len())
            .collect::<String>()
            .trim()
            .to_string();
    }

    if result.contains('`') {
        check_result.replaced_back_quote = true;
        result = result.replace("`", "'");
    }

    (result, check_result)
}

impl CheckResult {
    pub fn format(&self) -> String {
        let mut message = String::new();

        if self.reduced_code_block {
            message.push_str("- コードブロックを取り除きました\n");
        }

        if self.replaced_back_quote {
            message.push_str("- \\`を'に置換しました\n");
        }

        message
    }
}

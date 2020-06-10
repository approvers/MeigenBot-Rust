const CODE_BLOCK: &str = "```";
pub struct CheckResult {
    pub replaced_back_quote: bool,
    pub replaced_blacklists: bool,
    pub reduced_codeblock: bool,
}

pub fn check_message(s: &str, blacklists: &Vec<String>) -> (String, CheckResult) {
    let mut result = s.to_string();
    let mut check_result = CheckResult {
        replaced_back_quote: false,
        replaced_blacklists: false,
        reduced_codeblock: false,
    };

    if result.starts_with(CODE_BLOCK) && result.ends_with(CODE_BLOCK) {
        check_result.reduced_codeblock = true;

        let result_len = result.chars().count();
        result = result
            .chars()
            .take(result_len - CODE_BLOCK.len())
            .skip(CODE_BLOCK.len())
            .collect::<String>()
            .trim()
            .to_string();
    }

    if result.contains("`") {
        check_result.replaced_back_quote = true;
        result = result.replace("`", "'");
    }

    for black_char in blacklists {
        if result.contains(black_char) {
            check_result.replaced_blacklists = true;
            result = result.replace(black_char, "");
        }
    }

    (result, check_result)
}

impl CheckResult {
    pub fn format(&self) -> String {
        let mut message = String::new();

        if self.reduced_codeblock {
            message.push_str("- コードブロックを取り除きました\n");
        }

        if self.replaced_back_quote {
            message.push_str("- \\`を'に置換しました\n");
        }

        if self.replaced_blacklists {
            message.push_str("- ブラックリストに追加されていた文字を空白に置換しました\n")
        }

        message
    }
}

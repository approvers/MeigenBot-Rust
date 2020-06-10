pub struct CheckResult {
    pub replaced_grace_accent: bool,
    pub replaced_blacklists: bool,
}

pub fn check_message(s: &str, blacklists: &Vec<String>) -> (String, CheckResult) {
    let mut result = s.to_string();
    let mut check_result = CheckResult {
        replaced_grace_accent: false,
        replaced_blacklists: false,
    };

    if s.contains("`") {
        check_result.replaced_grace_accent = true;
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

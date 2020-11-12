const SPACE: char = ' ';
const FULL_WIDTH_SPACE: char = '　';

const BASE_COMMAND: &str = "g!meigen";

pub struct ParsedMessage {
    pub sub_command: Option<String>,
    pub raw_content: String,
    pub raw_args: String,
    pub args: Vec<String>,
}

// メッセージをパースする。
// もしこのBotのコマンド呼び出し形式 (g!meigen ...) に一致していなければNone、一致していればSome(ParsedMessage)
pub fn parse_message(message: &str) -> Option<ParsedMessage> {
    let content = message.trim().to_string();

    let mut splitted = content
        .split(SPACE)
        .flat_map(|x| x.split(FULL_WIDTH_SPACE))
        .filter(|x| !x.trim().is_empty())
        .map(|x| x.to_string());

    if content.is_empty() || splitted.next().unwrap() != BASE_COMMAND {
        return None;
    }

    let sub_command = splitted.next();

    let args = splitted.collect::<Vec<String>>();

    let raw_args = content
        .chars()
        .skip(BASE_COMMAND.chars().count() + 1)
        .skip(sub_command.as_ref().map_or(0, |x| x.chars().count()) + 1)
        .collect::<String>()
        .trim()
        .to_string();

    Some(ParsedMessage {
        sub_command,
        raw_content: content,
        raw_args,
        args,
    })
}

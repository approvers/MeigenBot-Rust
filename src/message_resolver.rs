use crate::botconfig::{BotConfig, MeigenEntry, RegisteredMeigen};

use serenity::model::channel::Message;

use crate::message_checker::check_message;

const BASE_COMMAND: &str = "g!meigen";
const MAKE_COMMAND: &str = "make";
const LIST_COMMAND: &str = "list";
const FROM_ID_COMMAND: &str = "id";
const RANDOM_COMMAND: &str = "random";
const BY_AUTHOR_COMMAND: &str = "author";
const STAT_COMMAND: &str = "status";
const HELP_COMMAND: &str = "help";
const DELETE_COMMAND: &str = "delete";

const MEIGEN_MAX_LENGTH: usize = 300;
const MESSAGE_MAX_LENGTH: usize = 500;

const TENSAI_BISYOUJYO_BOT_ID: u64 = 688788399275901029;
const KAWAEMON_ID: u64 = 391857452360007680;

const SPACE: char = ' ';
const FULL_WIDTH_SPACE: char = '　';

pub struct MessageResolver {
    config: BotConfig,
}

struct ParsedMessage {
    raw_content: String,
    raw_args: String,
    args: Vec<String>,
}

pub struct CommandUsageError(String);
impl std::fmt::Display for CommandUsageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.0)
    }
}

type SolveResult = Result<Option<String>, CommandUsageError>;

impl MessageResolver {
    pub fn new(config: BotConfig) -> Self {
        Self { config }
    }

    pub fn solve(&mut self, message: &Message) -> SolveResult {
        let content = message.content.trim().to_string();

        let splitted = content
            .split(SPACE)
            .flat_map(|x| x.split(FULL_WIDTH_SPACE))
            .filter(|x| !x.trim().is_empty())
            .map(|x| x.to_string())
            .collect::<Vec<String>>();

        if content.is_empty() || splitted[0] != BASE_COMMAND {
            return Ok(None);
        }

        let sub_command = {
            let temp = splitted.get(1);
            match temp {
                Some(t) => t.to_ascii_lowercase(),
                None => return self.help(None),
            }
        };

        let args = splitted.iter().skip(2).cloned().collect::<Vec<String>>();

        let raw_args = content
            .chars()
            .skip(BASE_COMMAND.chars().count() + 1)
            .skip(sub_command.chars().count() + 1)
            .collect::<String>()
            .trim()
            .to_string();

        let parsed = ParsedMessage {
            raw_content: content,
            raw_args,
            args,
        };

        if message.author.id == KAWAEMON_ID {
            if let DELETE_COMMAND = sub_command.as_str() {
                return self.delete_meigen(parsed);
            }
        }

        match sub_command.as_str() {
            MAKE_COMMAND => self.make_meigen(parsed),
            LIST_COMMAND => self.list_meigen(parsed),
            FROM_ID_COMMAND => self.from_id_meigen(parsed),
            RANDOM_COMMAND => self.random_meigen(parsed),
            BY_AUTHOR_COMMAND => self.author_meigen(parsed),
            STAT_COMMAND => self.stat_meigen(),
            HELP_COMMAND => self.help(None),
            _ => self.help(None),
        }
    }

    fn register_meigen(
        &mut self,
        author: String,
        meigen: String,
    ) -> Result<&RegisteredMeigen, CommandUsageError> {
        let entry = MeigenEntry::new(author, meigen, self.config.max_meigen_length)
            .map_err(|x| CommandUsageError(x.to_string()))?;

        self.config
            .push_new_meigen(entry)
            .map_err(|x| CommandUsageError(format!("ファイル保存に失敗しました: {}", x)))
    }

    fn make_meigen(&mut self, message: ParsedMessage) -> SolveResult {
        if message.args.len() <= 1 {
            return self.help(Some("引数が足りないよ"));
        }

        let author = message.args.get(0).unwrap().clone();
        let (meigen, checked_result) = {
            let author_skipcount = message.raw_args.find(&author).unwrap() + author.chars().count();
            let content = message
                .raw_args
                .trim()
                .chars()
                .skip(author_skipcount)
                .collect::<String>();

            check_message(content.trim(), &self.config.blacklist)
        };

        let registered_meigen = self.register_meigen(author, meigen)?;

        let mut message = String::new();
        message += &checked_result.format();
        message += &registered_meigen.format();

        Ok(Some(message))
    }

    fn from_id_meigen(&self, message: ParsedMessage) -> SolveResult {
        if message.args.is_empty() {
            return self.help(Some("引数が足りないよ"));
        }

        let id = message.args[0]
            .parse()
            .map_err(|x| CommandUsageError(format!("第一引数が数値じゃないよ: {}", x)))?;

        match self.config.meigens.iter().find(|x| x.id() == id) {
            Some(meigen) => Ok(Some(meigen.format())),
            None => Err(CommandUsageError("そんなIDの名言はないよ".into())),
        }
    }

    fn list_meigen(&self, message: ParsedMessage) -> SolveResult {
        const LIST_MEIGEN_DEFAULT_COUNT: i32 = 5;
        const LIST_MEIGEN_DEFAULT_PAGE: i32 = 1;

        // 表示する数
        let show_count = parse_or(LIST_MEIGEN_DEFAULT_COUNT, message.args.get(0))?;
        let page = parse_or(LIST_MEIGEN_DEFAULT_PAGE, message.args.get(1))?;
        let meigen = self
            .config
            .meigens
            .iter()
            .collect::<Vec<&RegisteredMeigen>>();

        let result = listify(meigen.as_slice(), show_count, page)?;

        Ok(Some(result))
    }

    fn author_meigen(&self, message: ParsedMessage) -> SolveResult {
        const LIST_MEIGEN_DEFAULT_COUNT: i32 = 5;
        const LIST_MEIGEN_DEFAULT_PAGE: i32 = 1;

        if message.args.len() == 0 {
            return self.help(Some("引数が足りないよ"));
        }

        let target_author = &message.args[0];
        let show_count = parse_or(LIST_MEIGEN_DEFAULT_COUNT, message.args.get(1))?;
        let page = parse_or(LIST_MEIGEN_DEFAULT_PAGE, message.args.get(2))?;

        let filtered = self
            .config
            .meigens
            .iter()
            .filter(|x| x.author().contains(target_author))
            .collect::<Vec<&RegisteredMeigen>>();

        let result = listify(filtered.as_slice(), show_count, page)?;

        Ok(Some(result))
    }

    fn random_meigen(&self, message: ParsedMessage) -> SolveResult {
        use rand::Rng;
        let count: usize =
            {
                if !message.args.is_empty() {
                    message.args.get(0).unwrap().parse().map_err(|x| {
                        CommandUsageError(format!("引数が正しい数値じゃないよ: {}", x))
                    })?
                } else {
                    1
                }
            };

        if count == 0 {
            return Err(CommandUsageError("数は0以上にしましょうね".into()));
        }

        let mut rng = rand::thread_rng();
        let mut result = String::new();

        for _ in 0..count {
            let index = rng.gen_range(0, self.config.meigens.len());
            result += &format!("{}\n", self.config.meigens[index].format());
        }

        if result.chars().count() > MESSAGE_MAX_LENGTH {
            return Err(CommandUsageError(
                "長すぎて表示できないよ。もっと数を少なくしてね。".into(),
            ));
        }

        Ok(Some(result))
    }

    fn stat_meigen(&self) -> SolveResult {
        let text = format!("合計名言数: {}個", self.config.meigens.len());
        Ok(Some(text))
    }

    fn help(&self, message: Option<&str>) -> SolveResult {
        //trim is not const fn...
        #[allow(non_snake_case)]
        let HELP_TEXT: &str = "
```asciidoc
= meigen-bot-rust =
g!meigen [subcommand] [args...]

= subcommands =
    help                            :: この文を出します
    make [作者] [名言]                :: 名言を登録します
    list [表示数=5] [ページ=1]         :: 名言をリスト表示します
    id [名言ID]                      :: 指定されたIDの名言を表示します
    author [作者] [表示数=5] [ページ=1] :: 指定された作者によって作成された名言を一覧表示します
    random [表示数=1]                 :: ランダムに名言を出します
    status                          :: 現在登録されてる名言の数を出します
    delete [名言ID]                  :: 指定されたIDの名言を削除します かわえもんにしか使えません
```
"
        .trim();

        match message {
            Some(x) => Ok(Some(format!("{}\n{}", x, HELP_TEXT))),
            None => Ok(Some(HELP_TEXT.into())),
        }
    }

    fn delete_meigen(&mut self, message: ParsedMessage) -> SolveResult {
        if message.args.is_empty() {
            return self.help(Some("引数が足りないよ"));
        }

        let id = message
            .args
            .get(0)
            .unwrap()
            .parse()
            .map_err(|x| CommandUsageError(format!("引数が数字じゃないよ: {}", x)))?;

        self.config
            .delete_meigen(id)
            .map(|_| Some("削除しました".into()))
            .map_err(|x| CommandUsageError(x.to_string()))
    }
}

#[inline]
fn parse_or(default: i32, text: Option<&String>) -> Result<i32, CommandUsageError> {
    match text {
        Some(num) => num
            .parse()
            .map_err(|x| CommandUsageError(format!("引数が正しい数値じゃないよ: {}", x))),
        None => Ok(default),
    }
}

#[inline]
fn listify(
    slice: &[&RegisteredMeigen],
    show_count: i32,
    page: i32,
) -> Result<String, CommandUsageError> {
    const MAX_LENGTH_PER_MEIGEN: usize = 50;

    let range = {
        use std::convert::TryInto;

        let meigens_end_index = slice.len() as i32;
        if meigens_end_index > show_count {
            let from: usize = {
                let temp = meigens_end_index - show_count - (show_count * (page - 1));
                temp.try_into().map_err(|x| {
                    CommandUsageError(format!(
                        "引数が正しい数値じゃないよ: {}, from was {}",
                        x, temp
                    ))
                })?
            };

            let to: usize = {
                let temp = meigens_end_index - (show_count * (page - 1));
                temp.try_into().map_err(|x| {
                    CommandUsageError(format!(
                        "引数が正しい数値じゃないよ: {}, to was {}",
                        x, temp
                    ))
                })?
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
        result = format!("{}\n{}", result, &formatted);
    }

    if result.is_empty() {
        return Err(CommandUsageError("一致するものがなかったよ...".into()));
    }

    if result.chars().count() > MESSAGE_MAX_LENGTH {
        return Err(CommandUsageError(
            "結果が長すぎて表示できないよ。もっと値を少なくしてね".into(),
        ));
    }

    Ok(result)
}

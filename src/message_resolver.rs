use crate::botconfig::{BotConfig, MeigenEntry, RegisteredMeigen};
use log::*;
use serenity::model::channel::Message;

use crate::message_checker::check_message;

const BASE_COMMAND: &str = "g!meigen";
const MAKE_COMMAND: &str = "make";
const LIST_COMMAND: &str = "list";
const FROM_ID_COMMAND: &str = "id";
const RANDOM_COMMAND: &str = "random";
const STAT_COMMAND: &str = "status";
const HELP_COMMAND: &str = "help";

const MEIGEN_MAX_LENGTH: usize = 300;

const TENSAI_BISYOUJYO_BOT_ID: u64 = 688788399275901029;

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

        if message.author.id == TENSAI_BISYOUJYO_BOT_ID {
            return self.copy(content);
        }

        let splitted = content
            .split(" ")
            .map(|x| x.to_string())
            .collect::<Vec<String>>();

        if content.is_empty() || splitted[0] != BASE_COMMAND {
            return Ok(None);
        }

        let sub_command = {
            let temp = splitted.get(1);
            if let None = temp {
                return self.help();
            }
            temp.unwrap().to_ascii_lowercase()
        };

        let args = splitted
            .iter()
            .skip(2)
            .map(|x| x.clone())
            .collect::<Vec<String>>();

        let raw_args = content
            .chars()
            .skip(BASE_COMMAND.len() + 1)
            .skip(sub_command.len() + 1)
            .collect::<String>()
            .trim()
            .to_string();

        let parsed = ParsedMessage {
            raw_content: content,
            raw_args,
            args,
        };

        match sub_command.as_str() {
            MAKE_COMMAND => self.make_meigen(parsed),
            LIST_COMMAND => self.list_meigen(parsed),
            FROM_ID_COMMAND => self.from_id_meigen(parsed),
            RANDOM_COMMAND => self.random_meigen(parsed),
            STAT_COMMAND => self.stat_meigen(),
            HELP_COMMAND => self.help(),
            _ => self.help(),
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
        let author = message.args.iter().next().unwrap().clone();
        let (meigen, checked_result) = {
            let author_len = author.chars().count();
            let content = message
                .raw_args
                .trim()
                .chars()
                .skip(author_len)
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
        if message.args.len() == 0 {
            return self.help();
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
        const LIST_MAX_LENGTH_PER_MEIGEN: usize = 50;
        const LIST_MAX_LENGTH: usize = 500;

        #[inline]
        fn parse_or(default: i32, text: Option<&String>) -> Result<i32, CommandUsageError> {
            match text {
                Some(num) => num
                    .parse()
                    .map_err(|x| CommandUsageError(format!("引数が正しい数値じゃないよ: {}", x))),
                None => Ok(default),
            }
        }

        // 表示する数
        let show_count = parse_or(LIST_MEIGEN_DEFAULT_COUNT, message.args.get(0))?;
        let page = parse_or(LIST_MEIGEN_DEFAULT_PAGE, message.args.get(1))?;

        let range = {
            use std::convert::TryInto;
            let meigens_end_index = self.config.meigens.len() as i32;
            let from: usize = (meigens_end_index - show_count + 1 - (show_count * (page - 1)) - 1)
                .try_into()
                .map_err(|x| CommandUsageError(format!("引数が正しい数値じゃないよ: {}", x)))?;

            let to: usize = (meigens_end_index - (show_count * (page - 1)))
                .try_into()
                .map_err(|x| CommandUsageError(format!("引数が正しい数値じゃないよ: {}", x)))?;

            from..to
        };

        let mut result = String::new();

        for index in range {
            let meigen = match self.config.meigens.get(index) {
                Some(m) => m,
                None => break,
            };

            let formatted = meigen.tidy_format(LIST_MAX_LENGTH_PER_MEIGEN);
            result = format!("{}\n{}", result, &formatted);
        }

        if result.is_empty() {
            return Err(CommandUsageError("一致するものがなかったよ...".into()));
        }

        if result.chars().count() > LIST_MAX_LENGTH {
            return Err(CommandUsageError(
                "結果が長すぎて表示できないよ。もっと値を少なくしてね".into(),
            ));
        }

        Ok(Some(result))
    }

    fn random_meigen(&self, _message: ParsedMessage) -> SolveResult {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let index = rng.gen_range(0, self.config.meigens.len());

        Ok(Some(self.config.meigens[index].format()))
    }

    fn stat_meigen(&self) -> SolveResult {
        let text = format!("合計名言数: {}個", self.config.meigens.len());
        Ok(Some(text))
    }

    fn help(&self) -> SolveResult {
        Ok(Some("未実装だカス ヘルプコマンドが来るよ".into()))
    }

    fn parse_noobest_meigen(content: &str) -> Option<(usize, String)> {
        let mut started = false;
        let mut count = 0;
        let mut meigen = String::new();
        for line in content.lines() {
            count += 1;
            if line.trim() == "```" {
                if !started {
                    started = true;
                    continue;
                }
                if started {
                    return Some((count, meigen));
                }
            }

            if started {
                meigen += &format!("{}\n", line);
            }
        }

        return None;
    }

    fn copy(&mut self, mut noobest_meigen: String) -> SolveResult {
        let mut ok_count = 0;
        let mut dup_count = 0;
        let mut err_count = 0;
        'main: loop {
            let parse_result = Self::parse_noobest_meigen(&noobest_meigen);
            if parse_result.is_none() {
                break 'main;
            }

            let (skip_count, meigen_content) = parse_result.unwrap();

            let line_count = meigen_content.lines().count();

            //最後の行だけ取り出して、---をスキップしたところが名前
            let author = meigen_content
                .lines()
                .last()
                .unwrap()
                .chars()
                .skip("    --- ".len())
                .collect::<String>()
                .trim()
                .to_string();

            let content = meigen_content
                .lines()
                .take(line_count - 1)
                .fold(String::new(), |a, b| format!("{}\n{}", a, b))
                .trim()
                .to_string();

            let duplicated = self
                .config
                .meigens
                .iter()
                .filter(|m| m.author() == author)
                .filter(|m| m.content() == content)
                .count()
                != 0;

            if duplicated {
                dup_count += 1;
            } else {
                let resiger_result = self.register_meigen(author, content);
                match resiger_result {
                    Ok(_) => ok_count += 1,
                    Err(e) => {
                        println!("登録失敗: {}", e);
                        err_count += 1;
                    }
                }
            }

            noobest_meigen = noobest_meigen
                .lines()
                .skip(skip_count)
                .fold(String::new(), |a, b| format!("{}\n{}", a, b));
        }

        if ok_count == 0 {
            return Ok(None);
        }

        let text = format!(
            "{}個インポートしました({}個エラー、{}個はすでに登録済み)",
            ok_count, err_count, dup_count
        );

        Ok(Some(text))
    }
}

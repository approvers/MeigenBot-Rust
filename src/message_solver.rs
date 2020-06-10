use crate::botconfig::{BotConfig, MeigenEntry};
use serenity::model::channel::Message;

use crate::message_checker::check_message;

const BASE_COMMAND: &str = "g!meigen";
const MAKE_COMMAND: &str = "make";
const LIST_COMMAND: &str = "list";
const RANDOM_COMMAND: &str = "random";
const STAT_COMMAND: &str = "status";
const HELP_COMMAND: &str = "help";

const MEIGEN_MAX_LENGTH: usize = 300;

pub struct MessageSolver {
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

impl MessageSolver {
    pub fn new(config: BotConfig) -> Self {
        Self { config }
    }

    pub fn solve(&mut self, message: &Message) -> SolveResult {
        let content = message.content.trim().to_string();
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
            RANDOM_COMMAND => self.random_meigen(parsed),
            STAT_COMMAND => self.stat_meigen(),
            HELP_COMMAND => self.help(),
            _ => self.help(),
        }
    }

    fn make_meigen(&mut self, message: ParsedMessage) -> SolveResult {
        let author = message.args.iter().next().unwrap().clone();
        let (meigen, checked_result) = {
            let temp = message.raw_args.trim();
            let author_len = author.chars().count();

            let temp = temp.chars().skip(author_len).collect::<String>();

            check_message(temp.trim(), &self.config.blacklist)
        };

        let entry = MeigenEntry::new(author, meigen, self.config.max_meigen_length)
            .map_err(|x| CommandUsageError(x.into_string()))?;

        let registered_meigen = self
            .config
            .push_new_meigen(entry)
            .map_err(|x| CommandUsageError(format!("ファイル保存に失敗しました: {}", x)))?;

        let mut message = String::new();
        message += &checked_result.format();
        message += &registered_meigen.format();

        Ok(Some(message))
    }

    fn list_meigen(&self, _message: ParsedMessage) -> SolveResult {
        unimplemented!()
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
}

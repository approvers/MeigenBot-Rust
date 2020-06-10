use crate::botconfig::{BotConfig, MeigenEntry};
use serenity::model::channel::Message;

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
        let author = message.args.iter().last().unwrap().clone();
        let meigen = {
            let temp = message.raw_args.trim();
            let temp_len = temp.chars().count();
            let author_len = author.chars().count();

            temp.chars().take(temp_len - author_len).collect::<String>()
        };

        let entry = MeigenEntry::new(author, meigen, self.config.max_meigen_length)
            .map_err(|x| CommandUsageError(x.into_string()))?;

        let result = entry.format();
        let _ = self.config.push_new_meigen(entry);

        Ok(Some(result))
    }

    fn list_meigen(&self, _message: ParsedMessage) -> SolveResult {
        unimplemented!()
    }

    fn random_meigen(&self, _message: ParsedMessage) -> SolveResult {
        unimplemented!()
    }

    fn stat_meigen(&self) -> SolveResult {
        unimplemented!()
    }

    fn help(&self) -> SolveResult {
        Ok(Some("未実装だカス".into()))
    }
}

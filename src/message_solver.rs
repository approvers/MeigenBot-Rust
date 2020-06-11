use crate::botconfig::{BotConfig, MeigenEntry, RegisteredMeigen, TooLongMeigenError};
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
            .map_err(|x| CommandUsageError(x.into_string()))?;

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

    fn list_meigen(&self, message: ParsedMessage) -> SolveResult {
        if message.args.len() <= 1 {
            return self.help();
        }

        let id = message.args[1]
            .parse()
            .map_err(|x| CommandUsageError(format!("第二引数が数値じゃないよ: {}", x)))?;

        match message.args[0].as_str() {
            "id" => {
                if let Some(m) = self.config.meigens.iter().find(|m| m.id() == id) {
                    Ok(Some(m.format().into()))
                } else {
                    Err(CommandUsageError("そのIDの名言は存在しません".into()))
                }
            }
            _ => self.help(),
        }
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

        let text = format!(
            "{}個インポートしました({}個エラー、{}個はすでに登録済み)",
            ok_count, err_count, dup_count
        );

        Ok(Some(text))
    }
}

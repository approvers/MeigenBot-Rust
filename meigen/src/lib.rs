mod commands;
pub mod db;
mod message_parser;

use crate::db::MeigenDatabase;
use crate::message_parser::ParsedMessage;

use async_trait::async_trait;
use interface::{TextBot, TextBotResult, TextMessage};
use std::sync::Arc;
use tokio::sync::RwLock;

const MAKE_COMMAND: &str = "make";
const LIST_COMMAND: &str = "list";
const FROM_ID_COMMAND: &str = "id";
const RANDOM_COMMAND: &str = "random";
const SEARCH_COMMAND: &str = "search";
const STAT_COMMAND: &str = "status";
const HELP_COMMAND: &str = "help";
const DELETE_COMMAND: &str = "delete";

#[derive(thiserror::Error, Debug)]
pub enum Error<TDatabaseError>
where
    TDatabaseError: std::error::Error + std::fmt::Display + Send + Sync + 'static,
{
    #[error("引数が足りません")]
    NotEnoughArgs,

    #[error("{actual_size}は長すぎません。。。？{limit}文字以下にしてください。。。")]
    TooLongMeigen { actual_size: usize, limit: usize },

    #[error("ID{id}を持つ名言は見つかりませんでした")]
    RequestedMeigenNotFound { id: i64 },

    #[error("指定された条件に合致する名言は見つかりませんでした")]
    NoMeigenHit,

    #[error("ヒットした名言が多すぎます。もっと検索条件を厳しくしてみてください")]
    TooManyMeigenHit,

    #[error("データベースへのアクセスで問題が起きました")]
    DatabaseError(#[source] TDatabaseError),

    #[error("{args_index}番目の引数を数字に変換できませんでした")]
    NumberParseFail {
        args_index: usize,
        source: std::num::ParseIntError,
    },

    #[error("変数のアンダーフローを検知しました。多分引数の数値が大きすぎます")]
    ArgsTooBigNumber,

    #[error("無効な検索サブコマンドです")]
    InvalidSearchSubCommand,

    #[error("このコマンドは管理者(かわえもん)にしか使えません")]
    YouAreNotKawaemon,
}

pub struct MeigenBot<D: MeigenDatabase> {
    db: Arc<RwLock<D>>,
}

impl<D: MeigenDatabase> MeigenBot<D> {
    pub fn new(db: D) -> Self {
        Self {
            db: Arc::new(RwLock::new(db)),
        }
    }
}

#[async_trait]
impl<D: MeigenDatabase> TextBot for MeigenBot<D> {
    type Error = Error<D::Error>;

    async fn on_message(&self, msg: TextMessage<'_>) -> TextBotResult<Self::Error> {
        match message_parser::parse_message(msg.content) {
            Some(m) => {
                let result = call_command(&self.db, m, msg.is_kawaemon).await;

                if let Ok(msg) = result {
                    return TextBotResult::Ok { msg };
                }

                let err = result.err().unwrap();
                match err {
                    Error::DatabaseError(_) => return TextBotResult::UnexpectedError(err),
                    e => return TextBotResult::ExpectedError(e),
                }
            }

            None => TextBotResult::NotMatch,
        }
    }
}

type CommandResult<D> = Result<String, Error<<D as MeigenDatabase>::Error>>;

pub(crate) async fn call_command<D>(
    db: &Arc<RwLock<D>>,
    message: ParsedMessage,
    is_kawaemon: bool,
) -> CommandResult<D>
where
    D: MeigenDatabase,
{
    let sub_command = {
        match message.sub_command.as_ref() {
            Some(s) => s,
            None => return commands::help::<D>(),
        }
    };

    if sub_command == DELETE_COMMAND {
        if !is_kawaemon {
            return Err(Error::YouAreNotKawaemon);
        }

        return commands::delete(db, message).await;
    }

    match sub_command.as_str() {
        MAKE_COMMAND => commands::make(db, message).await,
        LIST_COMMAND => commands::list(db, message).await,
        FROM_ID_COMMAND => commands::id(db, message).await,
        RANDOM_COMMAND => commands::random(db, message).await,
        SEARCH_COMMAND => commands::search(db, message).await,
        STAT_COMMAND => commands::status(db).await,
        HELP_COMMAND => commands::help::<D>(),
        _ => commands::help::<D>(),
    }
}

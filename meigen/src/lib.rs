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
const EXPORT_COMMAND: &str = "export";

#[derive(thiserror::Error, Debug)]
pub enum Error {
    //
    // Expected Errors
    //
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

    #[error("{args_index}番目の引数を数字に変換できませんでした")]
    NumberParseFail {
        args_index: usize,
        source: std::num::ParseIntError,
    },

    #[error("変数のアンダーフローを検知しました。多分引数の数値が大きすぎます")]
    ArgsTooBigNumber,

    #[error("無効な検索サブコマンドです")]
    InvalidSearchSubCommand,

    #[error("無効なエクスポート形式です。`json`か`yaml`が使えます")]
    InvalidExportFormat,

    #[error("このコマンドは管理者(かわえもん)にしか使えません")]
    YouAreNotKawaemon,

    //
    // Unexpected Errors
    //
    #[error("データベースへのアクセスで予期せぬ問題が起きました")]
    DatabaseError(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("エクスポート中に予期せぬ問題が発生しました")]
    ExportError(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl Into<TextBotResult<Self>> for Error {
    fn into(self) -> TextBotResult<Self> {
        match self {
            Error::DatabaseError(_) | Error::ExportError(_) => TextBotResult::UnexpectedError(self),

            _ => TextBotResult::ExpectedError(self),
        }
    }
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
    type Error = Error;

    async fn on_message(&self, msg: TextMessage<'_>) -> TextBotResult<Self::Error> {
        match message_parser::parse_message(msg.content) {
            Some(m) => call_command(&self.db, m, msg.is_kawaemon).await,

            None => TextBotResult::NotMatch,
        }
    }
}

type CommandResult = Result<String, Error>;

pub(crate) async fn call_command<D>(
    db: &Arc<RwLock<D>>,
    message: ParsedMessage,
    is_kawaemon: bool,
) -> TextBotResult<Error>
where
    D: MeigenDatabase,
{
    let sub_command = {
        match message.sub_command.as_ref() {
            Some(s) => s,
            None => return commands::help().into_textbot_result(),
        }
    };

    if sub_command == DELETE_COMMAND {
        if !is_kawaemon {
            return TextBotResult::ExpectedError(Error::YouAreNotKawaemon);
        }

        return commands::delete(db, message).await.into_textbot_result();
    }

    match sub_command.as_str() {
        MAKE_COMMAND => commands::make(db, message).await.into_textbot_result(),
        LIST_COMMAND => commands::list(db, message).await.into_textbot_result(),
        FROM_ID_COMMAND => commands::id(db, message).await.into_textbot_result(),
        RANDOM_COMMAND => commands::random(db, message).await.into_textbot_result(),
        SEARCH_COMMAND => commands::search(db, message).await.into_textbot_result(),
        EXPORT_COMMAND => commands::export(db, message).await.into_textbot_result(),
        STAT_COMMAND => commands::status(db).await.into_textbot_result(),
        HELP_COMMAND => commands::help().into_textbot_result(),
        _ => TextBotResult::NotMatch,
    }
}

trait CommandResultExt {
    fn into_textbot_result(self) -> TextBotResult<Error>;
}

impl CommandResultExt for Result<String, Error> {
    fn into_textbot_result(self) -> TextBotResult<Error> {
        match self {
            Ok(msg) => TextBotResult::Ok { msg, files: None },
            Err(e) => e.into(),
        }
    }
}

impl CommandResultExt for Result<commands::export::MessageWithFile, Error> {
    fn into_textbot_result(self) -> TextBotResult<Error> {
        match self {
            Ok(msg) => TextBotResult::Ok {
                msg: msg.msg,
                files: Some(vec![msg.file]),
            },

            Err(e) => e.into(),
        }
    }
}

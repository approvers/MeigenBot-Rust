use crate::commands;
use crate::db::MeigenDatabase;
use crate::message_parser;

const MAKE_COMMAND: &str = "make";
const LIST_COMMAND: &str = "list";
const FROM_ID_COMMAND: &str = "id";
const RANDOM_COMMAND: &str = "random";
const BY_AUTHOR_COMMAND: &str = "author";
const STAT_COMMAND: &str = "status";
const HELP_COMMAND: &str = "help";
const DELETE_COMMAND: &str = "delete";

// ParsedMessageから、それぞれのコマンド処理を呼び出し、その結果を返す
pub fn call_command(
    db: &mut impl MeigenDatabase,
    message: message_parser::ParsedMessage,
    is_admin: bool,
) -> commands::Result {
    let sub_command = {
        match message.sub_command.as_ref() {
            Some(s) => s,
            None => return commands::help(),
        }
    };

    if is_admin && sub_command == DELETE_COMMAND {
        return commands::delete(db, message);
    }

    match sub_command.as_str() {
        MAKE_COMMAND => commands::make(db, message),
        LIST_COMMAND => commands::list(db, message),
        FROM_ID_COMMAND => commands::id(db, message),
        RANDOM_COMMAND => commands::random(db, message),
        BY_AUTHOR_COMMAND => commands::author(db, message),
        STAT_COMMAND => commands::status(db),
        HELP_COMMAND => commands::help(),
        _ => commands::help(),
    }
}

use crate::commands::Error;
use crate::commands::Result;
use crate::db::Database;
use crate::message_parser::ParsedMessage;

pub fn delete(db: &mut impl Database, message: ParsedMessage) -> Result {
    if message.args.is_empty() {
        return Err(Error::not_enough_args());
    }

    let id = message
        .args
        .get(0)
        .unwrap()
        .parse()
        .map_err(|e| Error::arg_num_parse_fail(1, e))?;

    db.delete_meigen(id)
        .map(|_| "削除しました".into())
        .map_err(|e| Error::save_failed(e))
}

use crate::commands::meigen_format;
use crate::commands::{Error, Result};
use crate::db::MeigenDatabase;
use crate::message_parser::ParsedMessage;

pub fn random(db: &impl MeigenDatabase, message: ParsedMessage) -> Result {
    use rand::Rng;
    let count: usize = {
        message
            .args
            .get(0)
            .map_or(Ok(1), |x| x.parse())
            .map_err(|e| Error::arg_num_parse_fail(1, e))?
    };

    let meigen_count = db.meigens().len();
    let mut rng = rand::thread_rng();
    let mut result = String::new();

    for _ in 0..count {
        let index = rng.gen_range(0, meigen_count);
        result += &format!("{}\n", meigen_format(&db.meigens()[index]));
    }

    Ok(result)
}

use {
    crate::{
        db::{FindOptions, MeigenDatabase},
        Synced,
    },
    anyhow::Result,
};

const LIST_LENGTH_LIMIT: usize = 400;

trait IterExt {
    fn fold_list(self) -> String;
}

impl<T, D> IterExt for T
where
    D: std::fmt::Display,
    T: Iterator<Item = D> + DoubleEndedIterator,
{
    fn fold_list(self) -> String {
        let (mut text, len) = self
            .rev()
            .fold((String::new(), 0), |(mut text, mut len), meigen| {
                if len < LIST_LENGTH_LIMIT {
                    let meigen = format!("{}\n", meigen);

                    text.insert_str(0, &meigen);
                    len += meigen.chars().count() + 1;
                }

                (text, len)
            });

        if len >= LIST_LENGTH_LIMIT {
            text.insert_str(0, "結果が長すぎたため、一部の名言は省略されました。\n");
        }

        text
    }
}

/// clamps number, returns clamped number and message which is sent to User
macro_rules! option {
    ({value: $value:ident, default: $default:literal, min: $min:literal, max: $max:literal $(,)?}) => {{
        match $value.unwrap_or($default) {
            n if n > $max => (
                $max,
                concat!(
                    stringify!($value),
                    "の値は大きすぎたため",
                    stringify!($max),
                    "に丸められました。\n"
                ),
            ),

            n if n < $min => (
                $max,
                concat!(
                    stringify!($value),
                    "の値は小さすぎたため",
                    stringify!($max),
                    "に丸められました。\n"
                ),
            ),

            n => (n, ""),
        }
    }};
}

mod help;
pub use help::help;

mod status;
pub use status::status;

mod random;
pub use random::random;

mod make;
pub use make::make;

async fn find(db: Synced<impl MeigenDatabase>, opt: FindOptions<'_>) -> Result<String> {
    Ok(db.read().await.find(opt).await?.into_iter().fold_list())
}

mod search;
pub use search::author::search_author;
pub use search::content::search_content;

mod list;
pub use list::list;

mod delete;
pub use delete::delete;

mod id;
pub use id::id;

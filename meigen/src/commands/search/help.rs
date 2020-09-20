use crate::db::MeigenDatabase;
use crate::CommandResult;

const HELP_TEXT: &str = "```asciidoc
= meigen-bot-rust (search help) =
g!meigen search [検索内容] [表示数=5] [ページ=1]

= 検索内容 =
author  :: 名言を発した人の名前から検索します
content :: 名言の内容から検索します

全ての検索コマンドが部分一致検索です。
```";

pub(crate) fn help<D>() -> CommandResult<D>
where
    D: MeigenDatabase,
{
    Ok(HELP_TEXT.to_string())
}
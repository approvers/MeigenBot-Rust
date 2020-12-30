use anyhow::Result;

pub async fn help() -> Result<String> {
    const HELP_TEXT: &str = "```asciidoc
= meigen-bot-rust =
g!meigen [subcommand] [args...]
= subcommands =
  help                                    :: この文を出します
  make [作者] [名言]                       :: 名言を登録します
  list [表示数=5] [ページ=1]                :: 名言をリスト表示します
  id [名言ID]                             :: 指定されたIDの名言を表示します
  search [検索内容] [表示数=5] [ページ=1]    :: 名言を検索します(g!meigen searchでヘルプを表示します)
  random [表示数=1]                       :: ランダムに名言を出します
  status                                 :: 現在登録されてる名言の数を出します
  delete [名言ID]                         :: 指定されたIDの名言を削除します かわえもんにしか使えません
```";

    Ok(HELP_TEXT.into())
}

use async_trait::async_trait;

pub struct FileEntry {
    pub name: String,
    pub data: Vec<u8>,
}

pub enum TextBotResult<E>
where
    E: std::error::Error + std::fmt::Display + std::fmt::Debug + Send + Sync + 'static,
{
    // 正常に処理された
    Ok {
        // 返すべきメッセージ
        msg: String,

        // ファイル
        files: Option<Vec<FileEntry>>,
    },

    // 正常に処理されたが処理すべきメッセージではなかった
    NotMatch,

    // 想定されたエラーが発生した
    // ユーザーが間違った引数を入力したなど
    ExpectedError(E),

    // 想定されていないエラーが発生した
    // データベースへの接続に失敗したなど
    UnexpectedError(E),
}

#[derive(Clone, Copy)]
pub struct TextMessage<'a> {
    pub content: &'a str,
    pub is_kawaemon: bool,
}

// TODO: Should provide database abstructions
#[async_trait]
pub trait TextBot {
    type Error: std::error::Error + std::fmt::Display + std::fmt::Debug + Send + Sync + 'static;

    async fn on_message(&self, msg: TextMessage<'_>) -> TextBotResult<Self::Error>;
}

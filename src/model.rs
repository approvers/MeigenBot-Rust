use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meigen {
    pub id: u32,
    pub author: String,
    pub content: String,
}

impl std::fmt::Display for Meigen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Meigen No.{}
```
{}
    --- {}
```",
            self.id, self.content, self.author
        )
    }
}

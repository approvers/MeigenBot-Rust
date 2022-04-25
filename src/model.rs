use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meigen {
    pub id: u32,
    pub author: String,
    pub content: String,
    pub loved_user_id: Vec<u64>,
}
impl Meigen {
    pub fn loves(&self) -> usize {
        self.loved_user_id.len()
    }

    pub fn is_loving(&self, user_id: u64) -> bool {
        self.loved_user_id.contains(&user_id)
    }
}

impl std::fmt::Display for Meigen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let loves = self.loves();
        let loves_description = if loves > 0 {
            format!("(♥ x{})", loves)
        } else {
            "".to_string()
        };

        write!(
            f,
            "Meigen No.{} {}
```
{}
    --- {}
```",
            self.id, loves_description, self.content, self.author
        )
    }
}

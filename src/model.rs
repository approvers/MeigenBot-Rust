use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meigen {
    pub id: u32,
    pub author: String,
    pub content: String,
    pub loved_user_id: Vec<u64>
}
impl Meigen {
    pub fn loves(&self) -> usize {
        self.loved_user_id.len()
    }

    pub fn is_loving(&self, user_id: u64) -> bool {
        self.loved_user_id.iter().any(|id| id == user_id)
    }

    pub fn love(&mut self, from: u64) -> Option<()> {
        if self.is_loving(from) {
            return None;
        }

        self.loved_user_id.push(from);
        Some(())
    }


    pub fn unlove(&mut self, from: u64) -> Option<()> {
        if !self.is_loving(from) {
            return None;
        }

        let position = self.loved_user_id.iter().position(|id| id == from)?;
        self.loved_user_id.remove(position);
        Some(())
    }
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

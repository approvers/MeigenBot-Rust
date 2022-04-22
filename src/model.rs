use serde::{Deserialize, Serialize};

type DiscordUserID = string;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meigen {
    pub id: u32,
    pub author: String,
    pub content: String,
    pub loved_user_id: Vec<DiscordUserID>
}
impl Meigen {
    pub fn loves(&self) -> usize {
        self.loved_discord_user_id.len()
    }

    pub fn is_loving(&self, user_id: DiscordUserID) -> bool {
        self.loved_discord_user_id.iter().any(|&id| id == discord_user_id)
    }

    pub fn love(&mut self, from: DiscordUserID) -> Option<()> {
        if self.is_loving(from) {
            return None;
        }

        self.loved_user_id.append(from);
        Some(())
    }


    pub fn unlove(&mut self, from: DiscordUserID) -> Option<()> {
        if !self.is_loving(from) {
            return None;
        }

        let position = self.loved_user_id.iter().position(|&id| id == from)?;
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

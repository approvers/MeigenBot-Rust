#[derive(Debug, Clone)]
pub struct MeigenID(u64);

#[derive(Debug, Clone)]
pub struct Meigen {
    pub id: MeigenID,
    pub author: String,
    pub content: String,
}

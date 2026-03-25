use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: u32,
    pub contains: String,
    #[serde(default)]
    pub contains_not: Option<String>,
    pub target_folder: String,
}

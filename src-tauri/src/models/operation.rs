use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOperation {
    pub original_path: PathBuf,
    pub new_path: PathBuf,
    pub copied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortResult {
    pub operations: Vec<FileOperation>,
    pub errors: Vec<String>,
    /// True when the user stopped the run early. Files already moved are still
    /// listed in `operations` so undo can reverse them.
    #[serde(default)]
    pub cancelled: bool,
}

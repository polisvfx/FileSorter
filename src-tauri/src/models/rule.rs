use serde::{Deserialize, Serialize};

/// Which part of the file the Contains expression is tested against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum MatchScope {
    /// Filename including extension — the historical behaviour.
    #[default]
    Name,
    /// Filename without its extension.
    Stem,
    /// Extension only, without the dot.
    Extension,
    /// Full path, so parent folder names can be matched too.
    Path,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: u32,
    pub contains: String,
    #[serde(default)]
    pub contains_not: Option<String>,
    pub target_folder: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub stop_on_match: bool,
    /// Treat Contains / Contains NOT as regular expressions. The `,` and `*`
    /// operators do not apply in this mode — regex has its own alternation.
    #[serde(default)]
    pub regex: bool,
    #[serde(default)]
    pub case_sensitive: bool,
    #[serde(default)]
    pub scope: MatchScope,
}

fn default_true() -> bool {
    true
}

use serde::{Deserialize, Serialize};

use crate::text::TextBlock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Content {
    /// Accepts legacy JSON tag `"Text"` from older clients.
    #[serde(alias = "Text")]
    Text(TextBlock),
}

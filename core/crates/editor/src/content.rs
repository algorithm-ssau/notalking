use serde::{Deserialize, Serialize};

use crate::text::TextBlock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ListMeta {
    #[serde(default)]
    pub level: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImageMeta {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoMeta {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Content {
    /// Accepts legacy JSON tag `"Text"` from older clients.
    #[serde(alias = "Text")]
    Text(TextBlock),
    OrderedListItem(TextBlock, ListMeta),
    UnorderedListItem(TextBlock, ListMeta),
    Image(ImageMeta),
    Video(VideoMeta),
}

impl Content {
    pub fn to_plain_text(&self) -> String {
        match self {
            Content::Text(tb)
            | Content::OrderedListItem(tb, _)
            | Content::UnorderedListItem(tb, _) => tb.to_plain_text(),
            Content::Image(_) | Content::Video(_) => String::new(),
        }
    }
}

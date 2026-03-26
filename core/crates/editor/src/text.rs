use std::collections::HashMap;

#[derive(Debug)]
pub enum DeleteDirection {
    Backward,
    Forward,
}

#[derive(Debug)]
pub struct FormattingStatus {
    pub is_set: bool,
    pub is_mixed: bool,
    pub value: Option<FormatValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FormatValue {
    Bool(bool),
    String(String),
}

#[warn(unused)]
pub struct TextBlock {
    chunks: Vec<Chunk>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Chunk {
    pub text: String,
    pub style: Style,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Style {
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub color: Option<String>,
}

impl Style {
    pub fn new() -> Self {
        Self {
            bold: None,
            italic: None,
            color: None,
        }
    }

    pub fn merge(&self, other: &Style) -> Style {
        Style {
            bold: other.bold.or(self.bold),
            italic: other.italic.or(self.italic),
            color: other.color.clone().or(self.color.clone()),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.bold.is_none() && self.italic.is_none() && self.color.is_none()
    }
}

impl TextBlock {
    pub fn new() -> Self {
        Self { chunks: Vec::new() }
    }

    pub fn from_text(text: &str) -> Self {
        if text.is_empty() {
            Self::new()
        } else {
            Self {
                chunks: vec![Chunk {
                    text: text.to_string(),
                    style: Style::new(),
                }],
            }
        }
    }

    pub fn len(&self) -> usize {
        self.chunks.iter().map(|c| c.text.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty() || self.len() == 0
    }

    pub fn to_plain_text(&self) -> String {
        self.chunks.iter().map(|c| &c.text).cloned().collect()
    }

    pub fn get_chunks(&self) -> &[Chunk] {
        &self.chunks
    }

    pub fn insert_text(&mut self, position: usize, text: &str, style: Style) {
        if text.is_empty() {
            return;
        }

        if self.is_empty() {
            self.chunks.push(Chunk {
                text: text.to_string(),
                style,
            });
            return;
        }

        if position >= self.len() {
            if let Some(last) = self.chunks.last_mut() {
                if last.style == style {
                    last.text.push_str(text);
                } else {
                    self.chunks.push(Chunk {
                        text: text.to_string(),
                        style,
                    });
                }
            }
            self.merge();
            return;
        }

        if let Some((idx, _, local)) = self.find_chunk_at(position) {
            self.split_at(idx, local);

            self.chunks.insert(
                idx + 1,
                Chunk {
                    text: text.to_string(),
                    style,
                },
            );
        }

        self.merge();
    }

    pub fn delete_range(&mut self, start: usize, end: usize) {
        if start >= end || start >= self.len() {
            return;
        }

        let end = end.min(self.len());

        if let Some((s_idx, _, s_local)) = self.find_chunk_at(start) {
            self.split_at(s_idx, s_local);
        }

        if let Some((e_idx, _, e_local)) = self.find_chunk_at(end) {
            self.split_at(e_idx, e_local);
        }

        let mut pos = 0;
        self.chunks.retain(|chunk| {
            let next = pos + chunk.text.len();
            let keep = !(pos >= start && next <= end);
            pos = next;
            keep
        });

        self.merge();
    }

    pub fn delete_at(&mut self, position: usize, direction: DeleteDirection) {
        match direction {
            DeleteDirection::Backward => {
                if position > 0 {
                    self.delete_range(position - 1, position);
                }
            }
            DeleteDirection::Forward => {
                if position < self.len() {
                    self.delete_range(position, position + 1);
                }
            }
        }
    }

    pub fn enable_formatting(&mut self, start: usize, end: usize, style: Style) {
        self.apply_formatting(start, end, style, true);
    }

    pub fn disable_formatting(&mut self, start: usize, end: usize, style: Style) {
        self.apply_formatting(start, end, style, false);
    }

    fn apply_formatting(&mut self, start: usize, end: usize, style: Style, enable: bool) {
        if start >= end {
            return;
        }

        if let Some((idx, _, local)) = self.find_chunk_at(start) {
            self.split_at(idx, local);
        }

        if let Some((idx, _, local)) = self.find_chunk_at(end) {
            self.split_at(idx, local);
        }

        let mut pos = 0;

        for chunk in &mut self.chunks {
            let next = pos + chunk.text.len();

            if next > start && pos < end {
                if enable {
                    chunk.style = chunk.style.merge(&style);
                } else {
                    if style.bold.is_some() {
                        chunk.style.bold = None;
                    }
                    if style.italic.is_some() {
                        chunk.style.italic = None;
                    }
                    if style.color.is_some() {
                        chunk.style.color = None;
                    }
                }
            }

            pos = next;
        }

        self.merge();
    }

    pub fn get_formatting(&self, start: usize, end: usize) -> HashMap<String, FormattingStatus> {
        let mut bold_vals = Vec::new();
        let mut italic_vals = Vec::new();
        let mut color_vals = Vec::new();

        let mut pos = 0;

        for chunk in &self.chunks {
            let next = pos + chunk.text.len();

            if next > start && pos < end {
                bold_vals.push(chunk.style.bold);
                italic_vals.push(chunk.style.italic);
                color_vals.push(chunk.style.color.clone());
            }

            pos = next;
        }

        let mut map = HashMap::new();

        map.insert("bold".into(), analyze_bool(bold_vals));
        map.insert("italic".into(), analyze_bool(italic_vals));
        map.insert("color".into(), analyze_string(color_vals));

        map
    }

    fn find_chunk_at(&self, pos: usize) -> Option<(usize, &Chunk, usize)> {
        let mut offset = 0;

        for (i, chunk) in self.chunks.iter().enumerate() {
            let next = offset + chunk.text.len();
            if pos < next {
                return Some((i, chunk, pos - offset));
            }
            offset = next;
        }

        None
    }

    fn split_at(&mut self, idx: usize, local: usize) {
        if idx >= self.chunks.len() {
            return;
        }

        let chunk = self.chunks[idx].clone();

        if local == 0 || local >= chunk.text.len() {
            return;
        }

        let left = chunk.text[..local].to_string();
        let right = chunk.text[local..].to_string();

        self.chunks[idx].text = left;

        self.chunks.insert(
            idx + 1,
            Chunk {
                text: right,
                style: chunk.style,
            },
        );
    }

    fn merge(&mut self) {
        let mut merged: Vec<Chunk> = Vec::new();

        for chunk in self.chunks.drain(..) {
            if let Some(last) = merged.last_mut() {
                if last.style == chunk.style {
                    last.text.push_str(&chunk.text);
                    continue;
                }
            }
            merged.push(chunk);
        }

        self.chunks = merged;
    }
}

fn analyze_bool(values: Vec<Option<bool>>) -> FormattingStatus {
    let set_vals: Vec<bool> = values.iter().flatten().copied().collect();
    let is_set = !set_vals.is_empty();
    let is_mixed = set_vals.len() > 1 && set_vals.iter().any(|v| *v != set_vals[0]);

    FormattingStatus {
        is_set,
        is_mixed,
        value: if is_mixed || !is_set {
            None
        } else {
            Some(FormatValue::Bool(set_vals[0]))
        },
    }
}

fn analyze_string(values: Vec<Option<String>>) -> FormattingStatus {
    let mut set_vals = Vec::new();

    for v in values.into_iter().flatten() {
        set_vals.push(v);
    }

    let is_set = !set_vals.is_empty();

    let is_mixed = set_vals.windows(2).any(|w| w[0] != w[1]);

    FormattingStatus {
        is_set,
        is_mixed,
        value: if is_mixed || !is_set {
            None
        } else {
            Some(FormatValue::String(set_vals[0].clone()))
        },
    }
}

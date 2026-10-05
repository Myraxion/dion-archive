use crate::error::DionError;
use std::borrow::Cow;

pub const TC_HEADER: &[u8] = b"\xEF\xBB\xBF\r\n";
pub const TC_TAIL: &str = "\x04\u{00c2}";

#[cfg(windows)]
pub const EOL: &str = "\r\n";
#[cfg(not(windows))]
pub const EOL: &str = "\n";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IonEntry {
    pub entry_name: String,
    pub raw_comment: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IonFile {
    pub entries: Vec<IonEntry>,
}

impl IonFile {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, DionError> {
        if !bytes.starts_with(TC_HEADER) {
            return Err(DionError::InvalidHeader);
        }

        let body = &bytes[TC_HEADER.len()..];
        let text = std::str::from_utf8(body).map_err(|_| DionError::InvalidUtf8)?;

        let mut entries = Vec::new();
        let mut seen_names = std::collections::HashSet::new();

        for (idx, line) in text.lines().enumerate() {
            let line_no = idx + 2;
            let trimmed = line.trim_end_matches(['\r', '\n']);
            if trimmed.is_empty() {
                continue;
            }

            let entry = parse_entry_line(trimmed)
                .map_err(|e| DionError::MalformedEntry(format!("line {line_no}: {e}")))?;
            let lower_name = entry.entry_name.to_lowercase();
            if !seen_names.insert(lower_name) {
                return Err(DionError::MalformedEntry(format!(
                    "line {line_no}: duplicate case-insensitive entry: {}",
                    entry.entry_name
                )));
            }
            entries.push(entry);
        }

        Ok(Self { entries })
    }

    fn find_index(&self, entry_name: &str) -> Option<usize> {
        if entry_name.is_ascii() {
            self.entries
                .iter()
                .position(|e| e.entry_name.eq_ignore_ascii_case(entry_name))
        } else {
            let target_lower = entry_name.to_lowercase();
            self.entries
                .iter()
                .position(|e| e.entry_name.to_lowercase() == target_lower)
        }
    }

    pub fn find_entry(&self, entry_name: &str) -> Option<&IonEntry> {
        self.find_index(entry_name).map(|idx| &self.entries[idx])
    }

    pub fn update_or_insert(&mut self, entry_name: &str, raw_comment: &str) {
        if let Some(idx) = self.find_index(entry_name) {
            self.entries[idx].raw_comment = raw_comment.to_string();
        } else {
            self.entries.push(IonEntry {
                entry_name: entry_name.to_string(),
                raw_comment: raw_comment.to_string(),
            });
        }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn remove(&mut self, entry_name: &str) -> bool {
        if let Some(pos) = self.find_index(entry_name) {
            self.entries.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>, DionError> {
        let mut bytes = Vec::from(TC_HEADER);
        for entry in &self.entries {
            let line = format_entry_line(&entry.entry_name, &entry.raw_comment)?;
            bytes.extend_from_slice(line.as_bytes());
        }
        Ok(bytes)
    }
}

pub fn parse_entry_line(line: &str) -> Result<IonEntry, DionError> {
    if line.starts_with('"') {
        let chars = line[1..].char_indices();
        let mut closing_idx = None;
        for (idx, ch) in chars {
            if ch == '"' {
                closing_idx = Some(idx + 1);
                break;
            }
        }

        let end = closing_idx.ok_or_else(|| {
            DionError::MalformedEntry(format!("unclosed quote in line: {line}"))
        })?;

        let name = &line[1..end];
        let remainder = &line[end + 1..];

        let raw_comment = if remainder.is_empty() {
            ""
        } else if let Some(stripped) = remainder.strip_prefix(' ') {
            stripped
        } else {
            return Err(DionError::MalformedEntry(format!(
                "expected space after quoted name in line: {line}"
            )));
        };

        Ok(IonEntry {
            entry_name: name.to_string(),
            raw_comment: raw_comment.to_string(),
        })
    } else if let Some((name, comment)) = line.split_once(' ') {
        Ok(IonEntry {
            entry_name: name.to_string(),
            raw_comment: comment.to_string(),
        })
    } else {
        Ok(IonEntry {
            entry_name: line.to_string(),
            raw_comment: String::new(),
        })
    }
}

pub fn format_entry_line(entry_name: &str, raw_comment: &str) -> Result<String, DionError> {
    let mut line = String::new();
    if entry_name.contains(' ') {
        line.push('"');
        line.push_str(entry_name);
        line.push('"');
    } else {
        line.push_str(entry_name);
    }
    line.push(' ');
    line.push_str(raw_comment);
    line.push_str("\r\n");

    if line.as_bytes().len() > 4096 {
        return Err(DionError::LineTooLong(
            entry_name.to_string(),
            line.as_bytes().len(),
        ));
    }
    Ok(line)
}

pub fn encode_comment(comment: &str) -> String {
    if !comment.contains(['\r', '\n']) {
        return comment.to_string();
    }

    let mut encoded = String::new();
    let mut chars = comment.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                encoded.push('\\');
                encoded.push('\\');
            }
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                encoded.push('\\');
                encoded.push('n');
            }
            '\n' => {
                encoded.push('\\');
                encoded.push('n');
            }
            other => {
                encoded.push(other);
            }
        }
    }
    encoded.push_str(TC_TAIL);
    encoded
}

pub fn decode_comment<'a>(raw: &'a str) -> Cow<'a, str> {
    if let Some(payload) = raw.strip_suffix(TC_TAIL) {
        let mut decoded = String::with_capacity(payload.len());
        let mut chars = payload.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '\\' {
                match chars.peek() {
                    Some(&'\\') => {
                        chars.next();
                        decoded.push('\\');
                    }
                    Some(&'n') => {
                        chars.next();
                        decoded.push_str(EOL);
                    }
                    _ => {
                        decoded.push('\\');
                    }
                }
            } else {
                decoded.push(ch);
            }
        }
        Cow::Owned(decoded)
    } else {
        Cow::Borrowed(raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_line_decode_borrowed() {
        let comment = "单行备注文本";
        let decoded = decode_comment(comment);
        assert!(matches!(decoded, Cow::Borrowed(_)));
        assert_eq!(decoded, "单行备注文本");
    }

    #[test]
    fn test_multi_line_decode_owned() {
        let raw = format!("line1\\nline2\\\\back{TC_TAIL}");
        let decoded = decode_comment(&raw);
        assert!(matches!(decoded, Cow::Owned(_)));
        assert_eq!(decoded, format!("line1{EOL}line2\\back"));
    }

    #[test]
    fn test_parse_entry_line_with_and_without_quotes() {
        let entry1 = parse_entry_line("hello.txt my comment").unwrap();
        assert_eq!(entry1.entry_name, "hello.txt");
        assert_eq!(entry1.raw_comment, "my comment");

        let entry2 = parse_entry_line("\"hello world.txt\" quoted comment").unwrap();
        assert_eq!(entry2.entry_name, "hello world.txt");
        assert_eq!(entry2.raw_comment, "quoted comment");
    }

    #[test]
    fn test_parse_invalid_header() {
        let res = IonFile::parse(b"invalid header");
        assert!(matches!(res, Err(DionError::InvalidHeader)));
    }

    #[test]
    fn test_parse_duplicate_entry_error() {
        let mut content = Vec::from(TC_HEADER);
        content.extend_from_slice(b"test.txt first\r\nTEST.TXT second\r\n");
        let res = IonFile::parse(&content);
        assert!(matches!(res, Err(DionError::MalformedEntry(_))));
    }

    #[test]
    fn test_encode_and_roundtrip_single_and_multi_line() {
        let single = "这是一个单行文本\\带有反斜杠";
        assert_eq!(encode_comment(single), single);

        let multi = "第一行\\带有反斜杠\r\n第二行";
        let encoded = encode_comment(multi);
        assert!(encoded.ends_with(TC_TAIL));
        let decoded = decode_comment(&encoded);
        assert_eq!(decoded, format!("第一行\\带有反斜杠{EOL}第二行"));
    }

    #[test]
    fn test_update_or_insert_preserves_order() {
        let mut ion = IonFile::new();
        ion.update_or_insert("first.txt", "c1");
        ion.update_or_insert("second.txt", "c2");
        ion.update_or_insert("FIRST.TXT", "c1_updated");

        assert_eq!(ion.entries.len(), 2);
        assert_eq!(ion.entries[0].raw_comment, "c1_updated");
        assert_eq!(ion.entries[1].raw_comment, "c2");
    }

    #[test]
    fn test_remove_and_is_empty() {
        let mut ion = IonFile::new();
        assert!(ion.is_empty());

        ion.update_or_insert("first.txt", "c1");
        ion.update_or_insert("second.txt", "c2");
        assert!(!ion.is_empty());

        // 大小写不敏感删除
        assert!(ion.remove("FIRST.TXT"));
        assert_eq!(ion.entries.len(), 1);
        assert_eq!(ion.entries[0].entry_name, "second.txt");

        // 删除不存在的条目返回 false
        assert!(!ion.remove("nonexistent.txt"));
        assert_eq!(ion.entries.len(), 1);

        // 删除最后一个条目后变为 empty
        assert!(ion.remove("second.txt"));
        assert!(ion.is_empty());
    }
}

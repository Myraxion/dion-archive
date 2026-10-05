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
    pub fn parse(bytes: &[u8]) -> Result<Self, DionError> {
        if !bytes.starts_with(TC_HEADER) {
            return Err(DionError::InvalidHeader);
        }

        let body = &bytes[TC_HEADER.len()..];
        let text = std::str::from_utf8(body).map_err(|_| DionError::InvalidUtf8)?;

        let mut entries = Vec::new();
        for line in text.lines() {
            let trimmed = line.trim_end_matches(['\r', '\n']);
            if trimmed.is_empty() {
                continue;
            }

            let entry = parse_entry_line(trimmed)?;
            entries.push(entry);
        }

        Ok(Self { entries })
    }

    pub fn find_entry(&self, target_name: &str) -> Option<&IonEntry> {
        let target_lower = target_name.to_lowercase();
        self.entries
            .iter()
            .find(|e| e.entry_name.to_lowercase() == target_lower)
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
}

//! Total Commander `descript.ion` 编解码模块。
//!
//! 提供 UTF-8 编码格式下的单行与多行备注解析、编码、转义及文件条目管理。

use std::borrow::Cow;

use crate::error::DionError;

pub const TC_HEADER: &[u8] = b"\xEF\xBB\xBF\r\n";
pub const TC_TAIL: &str = "\x04\u{00c2}";

#[cfg(windows)]
pub const EOL: &str = "\r\n";
#[cfg(not(windows))]
pub const EOL: &str = "\n";

/// 表示 `descript.ion` 文件中的单个文件/目录备注条目。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IonEntry {
    /// 目标文件名或目录名（若含空格则物理存储时带引号）
    pub entry_name: String,
    /// 原始编码形式的备注内容（多行可能含 `\n` 转义和结束标志）
    pub raw_comment: String,
}

/// 维护一个 `descript.ion` 文件中所有条目的内存模型。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IonFile {
    pub entries: Vec<IonEntry>,
}

impl IonFile {
    #[must_use]
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

            let physical_len = trimmed.len() + 2;
            let entry = parse_entry_line(trimmed)
                .map_err(|e| DionError::MalformedEntry(format!("line {line_no}: {e}")))?;

            if physical_len > 4096 {
                return Err(DionError::LineTooLong(entry.entry_name, physical_len));
            }
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

/// 解析单行 `descript.ion` 条目。
///
/// # Errors
///
/// - 如果引号未闭合或引号后缺少空格分隔符，返回 [`DionError::MalformedEntry`]。
pub fn parse_entry_line(line: &str) -> Result<IonEntry, DionError> {
    if let Some(stripped_line) = line.strip_prefix('"') {
        let chars = stripped_line.char_indices();
        let mut closing_idx = None;
        for (idx, ch) in chars {
            if ch == '"' {
                closing_idx = Some(idx);
                break;
            }
        }

        let end = closing_idx
            .ok_or_else(|| DionError::MalformedEntry(format!("unclosed quote in line: {line}")))?;

        let name = &stripped_line[..end];
        let remainder = &stripped_line[end + 1..];

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

/// 将条目格式化为写入 `descript.ion` 的单行物理存储文本。
///
/// # Errors
///
/// - 如果格式化后的单行字节数超过 4096 字节，返回 [`DionError::LineTooLong`]。
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

    if line.len() > 4096 {
        return Err(DionError::LineTooLong(entry_name.to_string(), line.len()));
    }
    Ok(line)
}

/// 对多行备注进行 Total Commander 兼容编码转义。
///
/// 单行文本保持原样，多行文本将 `\` 转义为 `\\`、换行转义为 `\n`，并在末尾追加 TC 结束标示符。
#[must_use]
pub fn encode_comment(comment: &str) -> String {
    if !comment.contains(['\r', '\n']) {
        return comment.to_string();
    }

    let mut encoded = String::with_capacity(comment.len() + 16);
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

/// 解码 `descript.ion` 中存储的原始备注。
///
/// 若为普通单行文本，零拷贝返回借用的 [`Cow::Borrowed`]；
/// 若为多行转义文本，则反转义并返回 [`Cow::Owned`]。
#[must_use]
pub fn decode_comment(raw: &str) -> Cow<'_, str> {
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

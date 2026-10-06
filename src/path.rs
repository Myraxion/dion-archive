//! 目标路径词法解析与归一化模块。
//!
//! 将用户输入的相对/绝对路径解析为父目录路径 (`parent_dir`) 与基名 (`entry_name`)，
//! 并确保父目录下的 `descript.ion` 可被正确定位。

use std::path::{Component, Path, PathBuf};

use crate::error::DionError;

/// 解析归一化后的路径目标信息。
#[derive(Debug, PartialEq, Eq)]
pub struct TargetResolved {
    /// 目标所在父目录路径（包含 `descript.ion`）
    pub parent_dir: PathBuf,
    /// 目标文件的纯条目名称（存储在 `descript.ion` 的 key）
    pub entry_name: String,
    /// 用户传入的原始路径字符串
    pub target_path: String,
}

/// 将用户传入的路径字符串纯词法归一化并解析为目标与父目录。
///
/// # Errors
///
/// - 如果路径为空字符串或经过 `..` 归一化后无法得到合法文件名，返回 [`DionError::InvalidPath`]。
pub fn resolve_target(raw_path: &str) -> Result<TargetResolved, DionError> {
    if raw_path.trim().is_empty() {
        return Err(DionError::InvalidPath(raw_path.to_string()));
    }

    let p = Path::new(raw_path);
    let mut normalized_parts: Vec<String> = Vec::new();
    let mut prefix: Option<PathBuf> = None;

    for comp in p.components() {
        match comp {
            Component::Prefix(p_comp) => {
                prefix = Some(PathBuf::from(p_comp.as_os_str()));
            }
            Component::RootDir => {
                if let Some(ref mut p) = prefix {
                    p.push(std::path::MAIN_SEPARATOR_STR);
                } else {
                    prefix = Some(PathBuf::from(std::path::MAIN_SEPARATOR_STR));
                }
            }
            Component::CurDir => {
                // 忽略 '.'
            }
            Component::ParentDir => {
                if let Some(last) = normalized_parts.last() {
                    if last != ".." {
                        normalized_parts.pop();
                        continue;
                    }
                }
                if prefix.is_none() {
                    normalized_parts.push("..".to_string());
                }
            }
            Component::Normal(name) => {
                normalized_parts.push(name.to_string_lossy().to_string());
            }
        }
    }

    let entry_name = normalized_parts
        .pop()
        .ok_or_else(|| DionError::InvalidPath(raw_path.to_string()))?;

    let mut parent_dir = prefix.unwrap_or_default();
    if normalized_parts.is_empty() {
        if parent_dir.as_os_str().is_empty() {
            parent_dir = PathBuf::from(".");
        }
    } else {
        for part in normalized_parts {
            parent_dir.push(part);
        }
    }

    Ok(TargetResolved {
        parent_dir,
        entry_name,
        target_path: raw_path.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_simple_filename() {
        let res = resolve_target("foo.txt").unwrap();
        assert_eq!(res.parent_dir, PathBuf::from("."));
        assert_eq!(res.entry_name, "foo.txt");
        assert_eq!(res.target_path, "foo.txt");
    }

    #[test]
    fn test_resolve_relative_path() {
        let res = resolve_target("sub/foo.txt").unwrap();
        assert_eq!(res.parent_dir, PathBuf::from("sub"));
        assert_eq!(res.entry_name, "foo.txt");
        assert_eq!(res.target_path, "sub/foo.txt");
    }

    #[test]
    fn test_resolve_redundant_dots() {
        let res = resolve_target("./sub/../sub/bar.txt").unwrap();
        assert_eq!(res.parent_dir, PathBuf::from("sub"));
        assert_eq!(res.entry_name, "bar.txt");
    }
}

use crate::cli::ListArgs;
use crate::codec::{decode_comment, IonFile, EOL};
use crate::error::DionError;
use serde::Serialize;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

#[derive(Serialize, Debug)]
pub struct ListItem {
    pub path: String,
    pub name: String,
    pub comment: String,
}

fn is_wide_char(ch: char) -> bool {
    matches!(
        ch,
        '\u{1100}'..='\u{115F}'
            | '\u{2329}'..='\u{232A}'
            | '\u{2E80}'..='\u{A4CF}'
            | '\u{AC00}'..='\u{D7A3}'
            | '\u{F900}'..='\u{FAFF}'
            | '\u{FE10}'..='\u{FE19}'
            | '\u{FE30}'..='\u{FE6F}'
            | '\u{FF01}'..='\u{FF60}'
            | '\u{FFE0}'..='\u{FFE6}'
            | '\u{1F300}'..='\u{1FAFF}'
    )
}

fn display_width(s: &str) -> usize {
    s.chars().map(|ch| if is_wide_char(ch) { 2 } else { 1 }).sum()
}

fn read_ion_file(dir: &Path) -> Result<Option<IonFile>, DionError> {
    let ion_path = dir.join("descript.ion");
    match fs::read(&ion_path) {
        Ok(bytes) => {
            let ion_file = IonFile::parse(&bytes)?;
            Ok(Some(ion_file))
        }
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(DionError::Io(e)),
    }
}

pub fn handle_list(args: ListArgs) -> Result<(), DionError> {
    let root_path = Path::new(&args.dir);
    let root_meta = fs::symlink_metadata(root_path).map_err(DionError::Io)?;

    if root_meta.file_type().is_symlink() {
        return Err(DionError::Usage(format!("'{}' is a symlink", args.dir)));
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x00000400;
        if root_meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(DionError::Usage(format!(
                "'{}' is a reparse point",
                args.dir
            )));
        }
    }

    if !root_meta.is_dir() {
        return Err(DionError::Usage(format!("'{}' is not a directory", args.dir)));
    }

    let mut items: Vec<ListItem> = Vec::new();

    if args.recursive {
        scan_recursive(root_path, "", &mut items)?;
    } else if let Some(ion_file) = read_ion_file(root_path)? {
        for entry in ion_file.entries {
            let comment = decode_comment(&entry.raw_comment).into_owned();
            items.push(ListItem {
                path: entry.entry_name.clone(),
                name: entry.entry_name,
                comment,
            });
        }
    }

    if args.json {
        let json_str = serde_json::to_string(&items)
            .map_err(|e| DionError::MalformedEntry(e.to_string()))?;
        print!("{json_str}{EOL}");
        return Ok(());
    }

    if items.is_empty() {
        return Ok(());
    }

    let max_col1_width = items
        .iter()
        .map(|item| display_width(&item.path))
        .max()
        .unwrap_or(0);
    let col2_offset = max_col1_width + 2;

    for item in &items {
        let item_width = display_width(&item.path);
        let padding = " ".repeat(col2_offset.saturating_sub(item_width));

        if item.comment.is_empty() {
            print!("{}{EOL}", item.path);
            continue;
        }

        let mut lines = item.comment.lines();
        if let Some(first_line) = lines.next() {
            print!("{}{}{}{EOL}", item.path, padding, first_line);
            let subsequent_padding = " ".repeat(col2_offset);
            for next_line in lines {
                print!("{}{}{EOL}", subsequent_padding, next_line);
            }
        }
    }

    Ok(())
}

fn scan_recursive(
    dir: &Path,
    rel_prefix: &str,
    items: &mut Vec<ListItem>,
) -> Result<(), DionError> {
    if let Some(ion_file) = read_ion_file(dir)? {
        for entry in ion_file.entries {
            let path_str = if rel_prefix.is_empty() {
                entry.entry_name.clone()
            } else {
                format!("{rel_prefix}/{}", entry.entry_name)
            };
            let comment = decode_comment(&entry.raw_comment).into_owned();
            items.push(ListItem {
                path: path_str,
                name: entry.entry_name,
                comment,
            });
        }
    }

    let read_dir = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(e) => return Err(DionError::Io(e)),
    };

    let mut subdirs: Vec<(String, std::path::PathBuf)> = Vec::new();

    for entry_res in read_dir {
        let entry = entry_res.map_err(DionError::Io)?;
        let entry_path = entry.path();

        let metadata = match fs::symlink_metadata(&entry_path) {
            Ok(m) => m,
            Err(e) => return Err(DionError::Io(e)),
        };

        if metadata.file_type().is_symlink() {
            continue;
        }

        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x00000400;
            if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                continue;
            }
        }

        if metadata.is_dir() {
            let dir_name = entry.file_name().to_string_lossy().to_string();
            subdirs.push((dir_name, entry_path));
        }
    }

    subdirs.sort_by(|a, b| a.0.cmp(&b.0));

    for (dir_name, subpath) in subdirs {
        let next_prefix = if rel_prefix.is_empty() {
            dir_name
        } else {
            format!("{rel_prefix}/{dir_name}")
        };
        scan_recursive(&subpath, &next_prefix, items)?;
    }

    Ok(())
}

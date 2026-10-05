use crate::error::DionError;
use std::fs;
use std::io::{IsTerminal, Read};
use std::path::Path;
use std::process::Command;

pub fn read_comment_from_reader<R: Read>(mut reader: R) -> Result<String, DionError> {
    let mut buffer = String::new();
    reader.read_to_string(&mut buffer).map_err(DionError::Io)?;
    Ok(buffer)
}

pub fn is_tty() -> bool {
    if std::env::var("DION_FORCE_TTY").map(|v| v == "1").unwrap_or(false) {
        return true;
    }
    std::io::stdin().is_terminal()
}

pub fn resolve_editor_command() -> (String, Vec<String>) {
    let editor = std::env::var("VISUAL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| std::env::var("EDITOR").ok().filter(|s| !s.trim().is_empty()))
        .unwrap_or_else(|| "notepad.exe".to_string());

    let path = Path::new(&editor);
    if path.is_file() {
        (editor, Vec::new())
    } else {
        let mut parts = editor.split_whitespace();
        let program = parts.next().unwrap_or("notepad.exe").to_string();
        let args: Vec<String> = parts.map(|s| s.to_string()).collect();
        (program, args)
    }
}

pub fn run_editor(initial_content: &str) -> Result<String, DionError> {
    if !is_tty() {
        return Err(DionError::Usage(
            "interactive editor (-e) requires a TTY terminal".to_string(),
        ));
    }

    let pid = std::process::id();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let temp_path = std::env::temp_dir().join(format!("dion_edit_{pid}_{now}.txt"));

    fs::write(&temp_path, initial_content).map_err(DionError::Io)?;

    let (prog, args) = resolve_editor_command();
    let mut cmd = if cfg!(windows)
        && (prog.to_lowercase().ends_with(".bat") || prog.to_lowercase().ends_with(".cmd"))
    {
        let mut c = Command::new("cmd.exe");
        c.arg("/C").arg(&prog);
        c
    } else {
        Command::new(&prog)
    };

    cmd.args(&args).arg(&temp_path);
    cmd.stdin(std::process::Stdio::inherit());
    cmd.stdout(std::process::Stdio::inherit());
    cmd.stderr(std::process::Stdio::inherit());

    let status = match cmd.status() {
        Ok(s) => s,
        Err(e) => {
            let _ = fs::remove_file(&temp_path);
            return Err(DionError::Io(e));
        }
    };

    if !status.success() {
        let _ = fs::remove_file(&temp_path);
        return Err(DionError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("editor '{prog}' exited with non-zero status: {status}"),
        )));
    }

    let edited = fs::read_to_string(&temp_path).map_err(DionError::Io)?;
    let _ = fs::remove_file(&temp_path);

    Ok(edited)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_comment_preserves_surrounding_whitespace_and_newlines() {
        let input_bytes = b"  \r\nHello\r\nWorld \r\n ";
        let res = read_comment_from_reader(&input_bytes[..]).unwrap();
        assert_eq!(res, "  \r\nHello\r\nWorld \r\n ");
    }

    #[test]
    fn test_read_comment_preserves_special_characters_and_emojis() {
        let text = "Special: \"quotes\" \\ backslash \\n literal 🚀🎉\n";
        let res = read_comment_from_reader(text.as_bytes()).unwrap();
        assert_eq!(res, text);
    }

    #[test]
    fn test_read_comment_invalid_utf8_fails() {
        let invalid_bytes = [0xff, 0xfe, 0xfd];
        let res = read_comment_from_reader(&invalid_bytes[..]);
        assert!(res.is_err());
    }

    #[test]
    fn test_resolve_editor_command_precedence() {
        // 当未设置环境变量时默认为 notepad.exe
        std::env::remove_var("VISUAL");
        std::env::remove_var("EDITOR");
        let (prog, args) = resolve_editor_command();
        assert_eq!(prog, "notepad.exe");
        assert!(args.is_empty());

        // 设置 EDITOR
        std::env::set_var("EDITOR", "my_editor --wait");
        let (prog, args) = resolve_editor_command();
        assert_eq!(prog, "my_editor");
        assert_eq!(args, vec!["--wait"]);

        // 设置 VISUAL 覆盖 EDITOR
        std::env::set_var("VISUAL", "visual_editor");
        let (prog, args) = resolve_editor_command();
        assert_eq!(prog, "visual_editor");
        assert!(args.is_empty());

        // 清理环境变量
        std::env::remove_var("VISUAL");
        std::env::remove_var("EDITOR");
    }
}

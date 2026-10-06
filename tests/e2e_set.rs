use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

#[cfg(windows)]
use std::os::windows::fs::MetadataExt;

#[test]
fn test_set_single_line_comment_and_hidden_attribute() {
    let dir = tempdir().unwrap();
    let target_file = dir.path().join("hello.txt");
    fs::write(&target_file, "content").unwrap();

    let mut cmd = Command::cargo_bin("dion").unwrap();
    cmd.current_dir(dir.path())
        .arg("set")
        .arg("hello.txt")
        .arg("这是一个单行备注")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    // 验证 descript.ion 是否存在且具备 Windows FILE_ATTRIBUTE_HIDDEN (0x2)
    let ion_path = dir.path().join("descript.ion");
    assert!(ion_path.exists());

    #[cfg(windows)]
    {
        let metadata = fs::metadata(&ion_path).unwrap();
        let attrs = metadata.file_attributes();
        assert_ne!(
            attrs & 0x0000_0002,
            0,
            "descript.ion must have FILE_ATTRIBUTE_HIDDEN attribute"
        );
    }

    // 验证通过 dion get 读取无损还原
    let mut get_cmd = Command::cargo_bin("dion").unwrap();
    get_cmd
        .current_dir(dir.path())
        .arg("get")
        .arg("hello.txt")
        .assert()
        .success()
        .stdout(predicate::str::diff("这是一个单行备注\r\n"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn test_set_multi_line_comment_compliance_and_lossless_roundtrip() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("multi target.txt"), "content").unwrap();
    let comment_payload = "第一行\r\n第二行\\带反斜杠\r\n第三行";

    let mut cmd = Command::cargo_bin("dion").unwrap();
    cmd.current_dir(dir.path())
        .arg("set")
        .arg("multi target.txt")
        .arg(comment_payload)
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    // 1. 验证 physical raw 格式合规性
    let mut raw_cmd = Command::cargo_bin("dion").unwrap();
    let expected_raw = "第一行\\n第二行\\\\带反斜杠\\n第三行\x04\u{00c2}\r\n";
    raw_cmd
        .current_dir(dir.path())
        .arg("get")
        .arg("multi target.txt")
        .arg("--raw")
        .assert()
        .success()
        .stdout(predicate::str::diff(expected_raw));

    // 2. 验证物理文件内容
    let ion_path = dir.path().join("descript.ion");
    let content = fs::read(&ion_path).unwrap();
    let expected_entry_line = format!("\"multi target.txt\" {expected_raw}");
    let file_text = String::from_utf8(content).unwrap();
    assert!(
        file_text.contains(&expected_entry_line),
        "file must contain properly quoted entry and escaped multi-line comment"
    );

    // 3. 验证 dion get 能够无损还原
    let mut get_cmd = Command::cargo_bin("dion").unwrap();
    get_cmd
        .current_dir(dir.path())
        .arg("get")
        .arg("multi target.txt")
        .assert()
        .success()
        .stdout(predicate::str::diff(
            "第一行\r\n第二行\\带反斜杠\r\n第三行\r\n",
        ))
        .stderr(predicate::str::is_empty());
}

#[test]
fn test_set_single_line_with_literal_escapes() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("script.bat"), "@echo off").unwrap();
    // 包含字面量反斜杠与 \n，但没有真实换行，必须作为单行处理且不被隐式转义
    let literal_comment = r"C:\Windows\System32\n_literal";

    let mut cmd = Command::cargo_bin("dion").unwrap();
    cmd.current_dir(dir.path())
        .arg("set")
        .arg("script.bat")
        .arg(literal_comment)
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    // raw 模式下依然无 \x04\u{00c2}
    let mut raw_cmd = Command::cargo_bin("dion").unwrap();
    raw_cmd
        .current_dir(dir.path())
        .arg("get")
        .arg("script.bat")
        .arg("--raw")
        .assert()
        .success()
        .stdout(predicate::str::diff(format!("{literal_comment}\r\n")));

    // get 还原
    let mut get_cmd = Command::cargo_bin("dion").unwrap();
    get_cmd
        .current_dir(dir.path())
        .arg("get")
        .arg("script.bat")
        .assert()
        .success()
        .stdout(predicate::str::diff(format!("{literal_comment}\r\n")));
}

#[test]
fn test_set_case_insensitive_in_place_update_and_order_preservation() {
    let dir = tempdir().unwrap();
    for f in ["alpha.txt", "beta.txt", "gamma.txt", "delta.txt"] {
        fs::write(dir.path().join(f), "content").unwrap();
    }

    let ion_path = dir.path().join("descript.ion");
    let mut initial = Vec::from(b"\xEF\xBB\xBF\r\n" as &[u8]);
    initial.extend_from_slice(b"alpha.txt \xE5\xA4\x87\xE6\xB3\xA81\r\n"); // 备注1
    initial.extend_from_slice(b"beta.txt \xE5\xA4\x87\xE6\xB3\xA82\r\n"); // 备注2
    initial.extend_from_slice(b"gamma.txt \xE5\xA4\x87\xE6\xB3\xA83\r\n"); // 备注3
    fs::write(&ion_path, initial).unwrap();

    // 1. 大小写不敏感原地更新 beta.txt (传入 BETA.TXT)
    let mut update_cmd = Command::cargo_bin("dion").unwrap();
    update_cmd
        .current_dir(dir.path())
        .arg("set")
        .arg("BETA.TXT")
        .arg("更新后的备注2")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    // 2. 新增条目 delta.txt
    let mut insert_cmd = Command::cargo_bin("dion").unwrap();
    insert_cmd
        .current_dir(dir.path())
        .arg("set")
        .arg("delta.txt")
        .arg("新增备注4")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    // 3. 验证物理行序完全保持：alpha -> beta (更新) -> gamma -> delta (追加)
    let content = fs::read(&ion_path).unwrap();
    assert!(content.starts_with(b"\xEF\xBB\xBF\r\n"));
    let body_text = String::from_utf8(content[5..].to_vec()).unwrap();
    let lines: Vec<&str> = body_text
        .lines()
        .map(|l| l.trim_end_matches(['\r', '\n']))
        .filter(|l| !l.is_empty())
        .collect();

    // 应该有 4 个 entry
    assert_eq!(lines.len(), 4);
    assert!(
        lines[0].starts_with("alpha.txt "),
        "Line 1 must remain alpha.txt"
    );
    assert!(
        lines[1].starts_with("beta.txt ") && lines[1].ends_with("更新后的备注2"),
        "Line 2 must be in-place updated beta.txt"
    );
    assert!(
        lines[2].starts_with("gamma.txt "),
        "Line 3 must remain gamma.txt"
    );
    assert!(
        lines[3].starts_with("delta.txt ") && lines[3].ends_with("新增备注4"),
        "Line 4 must be newly appended delta.txt"
    );

    // 4. 验证 dion get beta.txt 读取无损还原
    let mut get_cmd = Command::cargo_bin("dion").unwrap();
    get_cmd
        .current_dir(dir.path())
        .arg("get")
        .arg("beta.txt")
        .assert()
        .success()
        .stdout(predicate::str::diff("更新后的备注2\r\n"));
}

#[test]
fn test_set_exceeding_4096_bytes_rejected_with_exit_code_3() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("huge.txt"), "content").unwrap();
    let ion_path = dir.path().join("descript.ion");
    let initial_content =
        b"\xEF\xBB\xBF\r\nexisting.txt \xE5\x8E\x9F\xE5\xA7\x8B\xE5\xA4\x87\xE6\xB3\xA8\r\n";
    fs::write(&ion_path, initial_content).unwrap();

    // 构造单行超长内容：entry_name + ' ' + payload + '\r\n' > 4096 bytes
    let huge_comment = "a".repeat(4095);

    let mut cmd = Command::cargo_bin("dion").unwrap();
    cmd.current_dir(dir.path())
        .arg("set")
        .arg("huge.txt")
        .arg(&huge_comment)
        .assert()
        .code(3)
        .stderr(predicate::str::contains("4096"));

    // 验证原文件未被篡改
    let current_content = fs::read(&ion_path).unwrap();
    assert_eq!(current_content, initial_content);
}

#[test]
fn test_set_malformed_header_rejected_exit_code_3() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("new.txt"), "content").unwrap();
    let ion_path = dir.path().join("descript.ion");
    let corrupt_content = b"ANSI header without BOM\r\nfile.txt comment\r\n";
    fs::write(&ion_path, corrupt_content).unwrap();

    let mut cmd = Command::cargo_bin("dion").unwrap();
    cmd.current_dir(dir.path())
        .arg("set")
        .arg("new.txt")
        .arg("new comment")
        .assert()
        .code(3)
        .stderr(predicate::str::contains("invalid header"));

    // 原文件原封未动
    assert_eq!(fs::read(&ion_path).unwrap(), corrupt_content);
}

#[test]
fn test_set_malformed_unclosed_quote_rejected_exit_code_3() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("target.txt"), "content").unwrap();
    let ion_path = dir.path().join("descript.ion");
    let mut corrupt_content = Vec::from(b"\xEF\xBB\xBF\r\n" as &[u8]);
    corrupt_content.extend_from_slice(b"\"unclosed name missing quote comment\r\n");
    fs::write(&ion_path, &corrupt_content).unwrap();

    let mut cmd = Command::cargo_bin("dion").unwrap();
    cmd.current_dir(dir.path())
        .arg("set")
        .arg("target.txt")
        .arg("test")
        .assert()
        .code(3)
        .stderr(predicate::str::contains("unclosed quote"));

    assert_eq!(fs::read(&ion_path).unwrap(), corrupt_content);
}

#[test]
fn test_set_duplicate_case_conflict_rejected_exit_code_3() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("other.txt"), "content").unwrap();
    let ion_path = dir.path().join("descript.ion");
    let mut corrupt_content = Vec::from(b"\xEF\xBB\xBF\r\n" as &[u8]);
    corrupt_content.extend_from_slice(b"file.txt c1\r\nFILE.TXT c2\r\n");
    fs::write(&ion_path, &corrupt_content).unwrap();

    let mut cmd = Command::cargo_bin("dion").unwrap();
    cmd.current_dir(dir.path())
        .arg("set")
        .arg("other.txt")
        .arg("test")
        .assert()
        .code(3)
        .stderr(predicate::str::contains("duplicate case-insensitive entry"));

    assert_eq!(fs::read(&ion_path).unwrap(), corrupt_content);
}

#[test]
fn test_set_relative_path_lexical() {
    let dir = tempdir().unwrap();
    let sub = dir.path().join("sub_dir");
    fs::create_dir(&sub).unwrap();
    fs::write(sub.join("item.txt"), "content").unwrap();


    let mut cmd = Command::cargo_bin("dion").unwrap();
    cmd.current_dir(dir.path())
        .arg("set")
        .arg("sub_dir/item.txt")
        .arg("子目录项目备注")
        .assert()
        .success();

    // 验证 descript.ion 位于 sub_dir 下而非根目录
    let root_ion = dir.path().join("descript.ion");
    assert!(!root_ion.exists());

    let sub_ion = sub.join("descript.ion");
    assert!(sub_ion.exists());

    // 验证通过 dion get 读取
    let mut get_cmd = Command::cargo_bin("dion").unwrap();
    get_cmd
        .current_dir(dir.path())
        .arg("get")
        .arg("sub_dir/item.txt")
        .assert()
        .success()
        .stdout(predicate::str::diff("子目录项目备注\r\n"));
}

#[test]
fn test_set_missing_args_exit_code_2() {
    // 缺少所有参数
    let mut cmd1 = Command::cargo_bin("dion").unwrap();
    cmd1.arg("set").assert().code(2);

    // 缺少 COMMENT
    let mut cmd2 = Command::cargo_bin("dion").unwrap();
    cmd2.arg("set").arg("target.txt").assert().code(2);
}

#[test]
fn test_set_stdin_via_dash_multi_line() {
    let dir = tempdir().unwrap();
    let target = dir.path().join("pipe_target.txt");
    fs::write(&target, "content").unwrap();

    let comment_payload = "第一行管道\r\n第二行管道\r\n第三行管道";

    let mut cmd = Command::cargo_bin("dion").unwrap();
    cmd.current_dir(dir.path())
        .arg("set")
        .arg("pipe_target.txt")
        .arg("-")
        .write_stdin(comment_payload)
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    // 验证 get 读取无损
    let mut get_cmd = Command::cargo_bin("dion").unwrap();
    get_cmd
        .current_dir(dir.path())
        .arg("get")
        .arg("pipe_target.txt")
        .assert()
        .success()
        .stdout(predicate::str::diff(format!("{comment_payload}\r\n")));
}

#[test]
fn test_set_stdin_via_flag_lossless_special_chars_and_whitespace() {
    let dir = tempdir().unwrap();
    let target = dir.path().join("special.txt");
    fs::write(&target, "content").unwrap();

    // 包含首尾空白、换行、双引号、反斜杠、Unicode 与 Emoji
    let comment_payload = "  \r\n\"Quotes\" & \\Backslash\\ 🚀 特殊符号 🎉\r\n  ";

    let mut cmd = Command::cargo_bin("dion").unwrap();
    cmd.current_dir(dir.path())
        .arg("set")
        .arg("special.txt")
        .arg("--stdin")
        .write_stdin(comment_payload)
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    // 验证 get 读取完全保留原始首尾空白与换行结构
    let mut get_cmd = Command::cargo_bin("dion").unwrap();
    get_cmd
        .current_dir(dir.path())
        .arg("get")
        .arg("special.txt")
        .assert()
        .success()
        .stdout(predicate::str::diff(format!("{comment_payload}\r\n")));
}

#[test]
fn test_set_stdin_empty_or_whitespace_triggers_unset() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("target.txt"), "content").unwrap();
    let ion_path = dir.path().join("descript.ion");

    // 先通过常规命令设置备注
    let mut setup_cmd = Command::cargo_bin("dion").unwrap();
    setup_cmd
        .current_dir(dir.path())
        .arg("set")
        .arg("target.txt")
        .arg("初始备注")
        .assert()
        .success();

    assert!(ion_path.exists());

    // 通过 stdin 传入全空白
    let mut clear_cmd = Command::cargo_bin("dion").unwrap();
    clear_cmd
        .current_dir(dir.path())
        .arg("set")
        .arg("target.txt")
        .arg("--stdin")
        .write_stdin("   \r\n\t  ")
        .assert()
        .success();

    // 验证条目已被 unset，文件自动删除
    assert!(!ion_path.exists());
}

#[test]
fn test_set_conflicting_sources_exit_code_2() {
    let mut cmd1 = Command::cargo_bin("dion").unwrap();
    cmd1.arg("set")
        .arg("file.txt")
        .arg("direct_comment")
        .arg("--stdin")
        .assert()
        .code(2);

    let mut cmd2 = Command::cargo_bin("dion").unwrap();
    cmd2.arg("set")
        .arg("file.txt")
        .arg("-")
        .arg("--stdin")
        .assert()
        .code(2);

    let mut cmd3 = Command::cargo_bin("dion").unwrap();
    cmd3.arg("set")
        .arg("file.txt")
        .arg("direct_comment")
        .arg("-e")
        .assert()
        .code(2);

    let mut cmd4 = Command::cargo_bin("dion").unwrap();
    cmd4.arg("set")
        .arg("file.txt")
        .arg("-")
        .arg("-e")
        .assert()
        .code(2);

    let mut cmd5 = Command::cargo_bin("dion").unwrap();
    cmd5.arg("set")
        .arg("file.txt")
        .arg("--stdin")
        .arg("-e")
        .assert()
        .code(2);
}

#[test]
fn test_set_editor_non_tty_rejected_exit_code_2() {
    let dir = tempdir().unwrap();
    let target = dir.path().join("editor_target.txt");
    fs::write(&target, "content").unwrap();

    let mut cmd = Command::cargo_bin("dion").unwrap();
    cmd.current_dir(dir.path())
        .arg("set")
        .arg("editor_target.txt")
        .arg("-e")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("TTY"));
}

#[test]
fn test_set_editor_non_tty_rejected_even_if_ion_corrupt_exit_code_2() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("file.txt"), "content").unwrap();
    let ion_path = dir.path().join("descript.ion");
    fs::write(&ion_path, b"corrupted header without BOM\r\n").unwrap();


    let mut cmd = Command::cargo_bin("dion").unwrap();
    cmd.current_dir(dir.path())
        .arg("set")
        .arg("file.txt")
        .arg("-e")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("TTY"));
}

#[test]
fn test_set_editor_interactive_flow_via_mock_editor() {
    let dir = tempdir().unwrap();
    let target = dir.path().join("file.txt");
    fs::write(&target, "file content").unwrap();

    // 创建一个 mock editor 脚本 (Windows .bat 批处理)
    // 脚本功能：接收临时文件路径作为第 1 个参数，向其写入指定内容
    let mock_editor = dir.path().join("mock_editor.bat");
    fs::write(
        &mock_editor,
        "@echo off\r\necho 编辑器写入的第一行> \"%~1\"\r\necho 编辑器写入的第二行>> \"%~1\"\r\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("dion").unwrap();
    cmd.current_dir(dir.path())
        .env("DION_FORCE_TTY", "1")
        .env("EDITOR", mock_editor.to_str().unwrap())
        .arg("set")
        .arg("file.txt")
        .arg("-e")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    // 验证 get 能还原编辑器写入的多行内容（含末尾保留换行）
    let mut get_cmd = Command::cargo_bin("dion").unwrap();
    get_cmd
        .current_dir(dir.path())
        .arg("get")
        .arg("file.txt")
        .assert()
        .success()
        .stdout(predicate::str::diff(
            "编辑器写入的第一行\r\n编辑器写入的第二行\r\n\r\n",
        ));
}

#[test]
fn test_set_editor_prefills_existing_comment_and_updates() {
    let dir = tempdir().unwrap();
    let target = dir.path().join("file.txt");
    fs::write(&target, "content").unwrap();

    // 1. 设置初始备注
    let mut init_cmd = Command::cargo_bin("dion").unwrap();
    init_cmd
        .current_dir(dir.path())
        .arg("set")
        .arg("file.txt")
        .arg("初始已有备注")
        .assert()
        .success();

    // 2. 创建 mock editor，在已有内容后追加文本
    let mock_editor = dir.path().join("append_editor.bat");
    fs::write(
        &mock_editor,
        "@echo off\r\necho.>> \"%~1\"\r\necho 追加的第二行>> \"%~1\"\r\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("dion").unwrap();
    cmd.current_dir(dir.path())
        .env("DION_FORCE_TTY", "1")
        .env("EDITOR", mock_editor.to_str().unwrap())
        .arg("set")
        .arg("file.txt")
        .arg("-e")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    // 3. 验证保留了原备注并成功合并追加内容
    let mut get_cmd = Command::cargo_bin("dion").unwrap();
    get_cmd
        .current_dir(dir.path())
        .arg("get")
        .arg("file.txt")
        .assert()
        .success()
        .stdout(predicate::str::diff("初始已有备注\r\n追加的第二行\r\n\r\n"));
}

#[test]
fn test_set_editor_clearing_content_triggers_unset() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("file.txt"), "content").unwrap();
    let ion_path = dir.path().join("descript.ion");

    // 1. 设置初始备注
    let mut init_cmd = Command::cargo_bin("dion").unwrap();
    init_cmd
        .current_dir(dir.path())
        .arg("set")
        .arg("file.txt")
        .arg("即将被清空的备注")
        .assert()
        .success();
    assert!(ion_path.exists());

    // 2. 创建清空临时文件的 mock editor
    let mock_editor = dir.path().join("clear_editor.bat");
    fs::write(&mock_editor, "@echo off\r\ntype nul > \"%~1\"\r\n").unwrap();

    let mut cmd = Command::cargo_bin("dion").unwrap();
    cmd.current_dir(dir.path())
        .env("DION_FORCE_TTY", "1")
        .env("EDITOR", mock_editor.to_str().unwrap())
        .arg("set")
        .arg("file.txt")
        .arg("-e")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    // 3. 验证文件被自动清理 (unset)
    assert!(!ion_path.exists());
}

#[test]
fn test_set_editor_does_not_lock_descript_ion_during_session() {
    let dir = tempdir().unwrap();
    let target = dir.path().join("file.txt");
    fs::write(&target, "file content").unwrap();

    // 1. 初始化 descript.ion
    let ion_path = dir.path().join("descript.ion");
    let mut initial = Vec::from(b"\xEF\xBB\xBF\r\n" as &[u8]);
    initial.extend_from_slice(b"file.txt original\r\n");
    fs::write(&ion_path, initial).unwrap();

    // 2. 创建 mock editor：在会话运行期间，外部可以直接读写目标 descript.ion
    let mock_editor = dir.path().join("concurrency_check_editor.bat");
    fs::write(
        &mock_editor,
        "@echo off\r\necho other.txt external_comment>> descript.ion\r\necho updated_content> \"%~1\"\r\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("dion").unwrap();
    cmd.current_dir(dir.path())
        .env("DION_FORCE_TTY", "1")
        .env("EDITOR", mock_editor.to_str().unwrap())
        .arg("set")
        .arg("file.txt")
        .arg("-e")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    // 3. 验证外部修改与编辑器修改均安全保留
    let mut get1 = Command::cargo_bin("dion").unwrap();
    get1.current_dir(dir.path())
        .arg("get")
        .arg("other.txt")
        .assert()
        .success()
        .stdout(predicate::str::diff("external_comment\r\n"));

    let mut get2 = Command::cargo_bin("dion").unwrap();
    get2.current_dir(dir.path())
        .arg("get")
        .arg("file.txt")
        .assert()
        .success()
        .stdout(predicate::str::diff("updated_content\r\n\r\n"));
}

#[test]
fn test_set_nonexistent_target_rejected_exit_code_1() {
    let dir = tempdir().unwrap();

    let mut cmd = Command::cargo_bin("dion").unwrap();
    cmd.current_dir(dir.path())
        .arg("set")
        .arg("nonexistent_file.txt")
        .arg("some comment")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("target not found: 'nonexistent_file.txt'"));

    // 验证未产生任何 descript.ion 文件
    let ion_path = dir.path().join("descript.ion");
    assert!(!ion_path.exists());
}

#[test]
fn test_set_empty_comment_on_nonexistent_target_rejected_exit_code_1() {
    let dir = tempdir().unwrap();

    // 哪怕传入空字符串备注，也不允许转发 unset，直接拦截
    let mut cmd = Command::cargo_bin("dion").unwrap();
    cmd.current_dir(dir.path())
        .arg("set")
        .arg("ghost_file.txt")
        .arg("")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("target not found: 'ghost_file.txt'"));

    let ion_path = dir.path().join("descript.ion");
    assert!(!ion_path.exists());
}

#[test]
fn test_set_stdin_empty_on_nonexistent_target_rejected_exit_code_1() {
    let dir = tempdir().unwrap();

    let mut cmd = Command::cargo_bin("dion").unwrap();
    cmd.current_dir(dir.path())
        .arg("set")
        .arg("ghost_pipe.txt")
        .arg("--stdin")
        .write_stdin("   \r\n")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("target not found: 'ghost_pipe.txt'"));
}

#[test]
fn test_set_editor_on_nonexistent_target_rejected_exit_code_1() {
    let dir = tempdir().unwrap();

    // 创建一个 mock editor 脚本，如果被调用就会生成 sentinel 文件
    let sentinel = dir.path().join("editor_ran.txt");
    let mock_editor = dir.path().join("mock_editor.bat");
    fs::write(
        &mock_editor,
        format!("@echo off\r\necho ran > \"{}\"\r\n", sentinel.display()),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("dion").unwrap();
    cmd.current_dir(dir.path())
        .env("DION_FORCE_TTY", "1")
        .env("EDITOR", mock_editor.to_str().unwrap())
        .arg("set")
        .arg("ghost_editor.txt")
        .arg("-e")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("target not found: 'ghost_editor.txt'"));

    // 验证编辑器根本没有被拉起
    assert!(!sentinel.exists());
}

#[test]
fn test_set_existing_directory_and_nonexistent_parent_dir() {
    let dir = tempdir().unwrap();

    // 1. 对存在的子目录设置备注：应当成功
    let sub = dir.path().join("my_folder");
    fs::create_dir(&sub).unwrap();

    let mut cmd1 = Command::cargo_bin("dion").unwrap();
    cmd1.current_dir(dir.path())
        .arg("set")
        .arg("my_folder")
        .arg("这是一个目录备注")
        .assert()
        .success();

    let mut get_cmd = Command::cargo_bin("dion").unwrap();
    get_cmd
        .current_dir(dir.path())
        .arg("get")
        .arg("my_folder")
        .assert()
        .success()
        .stdout(predicate::str::diff("这是一个目录备注\r\n"));

    // 2. 对不存在的父目录下的文件设置备注：应当返回退出码 1
    let mut cmd2 = Command::cargo_bin("dion").unwrap();
    cmd2.current_dir(dir.path())
        .arg("set")
        .arg("no_such_folder/file.txt")
        .arg("备注")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("target not found: 'no_such_folder/file.txt'"));
}





use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

const TC_HEADER: &[u8] = b"\xEF\xBB\xBF\r\n";
const TC_TAIL: &str = "\x04\u{00c2}";

#[test]
fn test_list_empty_directory_exit_code_0() {
    let dir = tempdir().unwrap();

    // 人类可读终端模式：无备注时输出为空，退出码 0
    let mut cmd = Command::cargo_bin("dion").unwrap();
    cmd.current_dir(dir.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());

    // JSON 模式：无备注时输出空数组 []，退出码 0
    let mut cmd_json = Command::cargo_bin("dion").unwrap();
    let assert_json = cmd_json
        .current_dir(dir.path())
        .arg("list")
        .arg("--json")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    let stdout_str = String::from_utf8(assert_json.get_output().stdout.clone()).unwrap();
    let val: serde_json::Value = serde_json::from_str(stdout_str.trim()).unwrap();
    assert_eq!(val, serde_json::json!([]));
}

#[test]
fn test_list_single_directory_aligned_two_columns_and_alias() {
    let dir = tempdir().unwrap();
    let ion_path = dir.path().join("descript.ion");
    let mut content = Vec::from(TC_HEADER);
    content.extend_from_slice("short.txt 短备注\r\n".as_bytes());
    content.extend_from_slice("a_much_longer_name.txt 长名称对应的备注\r\n".as_bytes());
    fs::write(&ion_path, content).unwrap();

    // 默认 list 命令
    let mut cmd = Command::cargo_bin("dion").unwrap();
    let assert = cmd
        .current_dir(dir.path())
        .arg("list")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    let stdout_str = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout_str.lines().collect();
    assert_eq!(lines.len(), 2);

    // 第一列宽度为 a_much_longer_name.txt 的长度 (22)，加 2 个空格分隔符，第二列起始位置为 24
    assert!(lines[0].starts_with("short.txt"));
    assert!(lines[0].ends_with("短备注"));
    assert!(lines[1].starts_with("a_much_longer_name.txt  长名称对应的备注"));

    // 短文件名的备注起始列应与长文件名对齐
    let col2_offset = lines[1].find("长名称对应的备注").unwrap();
    assert_eq!(lines[0].find("短备注").unwrap(), col2_offset);

    // 别名 ls 行为一致
    let mut cmd_ls = Command::cargo_bin("dion").unwrap();
    cmd_ls
        .current_dir(dir.path())
        .arg("ls")
        .assert()
        .success()
        .stdout(predicate::str::diff(stdout_str));
}

#[test]
fn test_list_multi_line_comment_indentation() {
    let dir = tempdir().unwrap();
    let ion_path = dir.path().join("descript.ion");
    let mut content = Vec::from(TC_HEADER);
    let multi = format!("multi.txt 第一行内容\\n第二行内容\\n第三行内容{TC_TAIL}\r\n");
    content.extend_from_slice(multi.as_bytes());
    content.extend_from_slice("single.txt 单行内容\r\n".as_bytes());
    fs::write(&ion_path, content).unwrap();

    let mut cmd = Command::cargo_bin("dion").unwrap();
    let assert = cmd
        .current_dir(dir.path())
        .arg("list")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    let stdout_str = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout_str.lines().collect();
    // multi.txt 占 3 行，single.txt 占 1 行，共 4 行
    assert_eq!(lines.len(), 4);

    let col2_offset = lines[0].find("第一行内容").unwrap();
    assert!(lines[1].starts_with(&" ".repeat(col2_offset)));
    assert!(lines[1].trim_start().starts_with("第二行内容"));
    assert!(lines[2].starts_with(&" ".repeat(col2_offset)));
    assert!(lines[2].trim_start().starts_with("第三行内容"));
    assert_eq!(lines[3].find("单行内容").unwrap(), col2_offset);
}

#[test]
fn test_list_json_flag_output() {
    let dir = tempdir().unwrap();
    let ion_path = dir.path().join("descript.ion");
    let mut content = Vec::from(TC_HEADER);
    let multi = format!("file1.txt 行一\\n行二{TC_TAIL}\r\n");
    content.extend_from_slice(multi.as_bytes());
    content.extend_from_slice("file2.txt 纯单行\r\n".as_bytes());
    fs::write(&ion_path, content).unwrap();

    let mut cmd = Command::cargo_bin("dion").unwrap();
    let assert = cmd
        .current_dir(dir.path())
        .arg("list")
        .arg("--json")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    let stdout_str = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let val: serde_json::Value = serde_json::from_str(stdout_str.trim()).unwrap();
    assert!(val.is_array());
    let arr = val.as_array().unwrap();
    assert_eq!(arr.len(), 2);

    assert_eq!(arr[0]["path"], "file1.txt");
    assert_eq!(arr[0]["name"], "file1.txt");
    #[cfg(windows)]
    assert_eq!(arr[0]["comment"], "行一\r\n行二");
    #[cfg(not(windows))]
    assert_eq!(arr[0]["comment"], "行一\n行二");

    assert_eq!(arr[1]["path"], "file2.txt");
    assert_eq!(arr[1]["name"], "file2.txt");
    assert_eq!(arr[1]["comment"], "纯单行");
}

#[test]
fn test_list_recursive_scan_with_slash_relative_path() {
    let dir = tempdir().unwrap();

    // 根目录 descript.ion
    let root_ion = dir.path().join("descript.ion");
    let mut root_content = Vec::from(TC_HEADER);
    root_content.extend_from_slice("root.txt 根目录备注\r\n".as_bytes());
    fs::write(&root_ion, root_content).unwrap();

    // 子目录 sub1
    let sub1 = dir.path().join("sub1");
    fs::create_dir(&sub1).unwrap();
    let sub1_ion = sub1.join("descript.ion");
    let mut sub1_content = Vec::from(TC_HEADER);
    sub1_content.extend_from_slice("child1.txt 子目录1备注\r\n".as_bytes());
    fs::write(&sub1_ion, sub1_content).unwrap();

    // 嵌套子目录 sub1/nested
    let nested = sub1.join("nested");
    fs::create_dir(&nested).unwrap();
    let nested_ion = nested.join("descript.ion");
    let mut nested_content = Vec::from(TC_HEADER);
    nested_content.extend_from_slice("deep.txt 深层备注\r\n".as_bytes());
    fs::write(&nested_ion, nested_content).unwrap();

    // 子目录 sub2
    let sub2 = dir.path().join("sub2");
    fs::create_dir(&sub2).unwrap();
    let sub2_ion = sub2.join("descript.ion");
    let mut sub2_content = Vec::from(TC_HEADER);
    sub2_content.extend_from_slice("child2.txt 子目录2备注\r\n".as_bytes());
    fs::write(&sub2_ion, sub2_content).unwrap();

    // 人类可读递归模式
    let mut cmd = Command::cargo_bin("dion").unwrap();
    let assert = cmd
        .current_dir(dir.path())
        .arg("list")
        .arg("-r")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    let stdout_str = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(stdout_str.contains("root.txt"));
    assert!(stdout_str.contains("sub1/child1.txt"));
    assert!(stdout_str.contains("sub1/nested/deep.txt"));
    assert!(stdout_str.contains("sub2/child2.txt"));

    // JSON 递归模式
    let mut cmd_json = Command::cargo_bin("dion").unwrap();
    let assert_json = cmd_json
        .current_dir(dir.path())
        .arg("list")
        .arg("-r")
        .arg("--json")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    let json_stdout = String::from_utf8(assert_json.get_output().stdout.clone()).unwrap();
    let val: serde_json::Value = serde_json::from_str(json_stdout.trim()).unwrap();
    let arr = val.as_array().unwrap();
    assert_eq!(arr.len(), 4);

    assert_eq!(arr[0]["path"], "root.txt");
    assert_eq!(arr[0]["name"], "root.txt");

    assert_eq!(arr[1]["path"], "sub1/child1.txt");
    assert_eq!(arr[1]["name"], "child1.txt");

    assert_eq!(arr[2]["path"], "sub1/nested/deep.txt");
    assert_eq!(arr[2]["name"], "deep.txt");

    assert_eq!(arr[3]["path"], "sub2/child2.txt");
    assert_eq!(arr[3]["name"], "child2.txt");
}

#[test]
fn test_list_with_explicit_dir_argument() {
    let dir = tempdir().unwrap();
    let sub = dir.path().join("my_dir");
    fs::create_dir(&sub).unwrap();

    let ion_path = sub.join("descript.ion");
    let mut content = Vec::from(TC_HEADER);
    content.extend_from_slice("test.txt 明确目录参数备注\r\n".as_bytes());
    fs::write(&ion_path, content).unwrap();

    let mut cmd = Command::cargo_bin("dion").unwrap();
    cmd.current_dir(dir.path())
        .arg("list")
        .arg("my_dir")
        .assert()
        .success()
        .stdout(predicate::str::contains("test.txt"))
        .stdout(predicate::str::contains("明确目录参数备注"));
}

#[test]
fn test_list_nonexistent_directory_exit_code_3() {
    let dir = tempdir().unwrap();

    let mut cmd = Command::cargo_bin("dion").unwrap();
    cmd.current_dir(dir.path())
        .arg("list")
        .arg("nonexistent_folder")
        .assert()
        .code(3)
        .stderr(predicate::str::contains("error:"));
}

#[test]
fn test_list_malformed_ion_file_exit_code_3() {
    let dir = tempdir().unwrap();
    let ion_path = dir.path().join("descript.ion");
    fs::write(&ion_path, b"invalid header content").unwrap();

    let mut cmd = Command::cargo_bin("dion").unwrap();
    cmd.current_dir(dir.path())
        .arg("list")
        .assert()
        .code(3)
        .stderr(predicate::str::contains("invalid header"));
}

#[cfg(windows)]
#[test]
fn test_list_recursive_does_not_cross_junction() {
    let dir = tempdir().unwrap();

    // 目标外部目录 outside_dir，里面有 descript.ion
    let outside_dir = dir.path().join("outside_dir");
    fs::create_dir(&outside_dir).unwrap();
    let outside_ion = outside_dir.join("descript.ion");
    let mut outside_content = Vec::from(TC_HEADER);
    outside_content.extend_from_slice("secret.txt 外部敏感备注\r\n".as_bytes());
    fs::write(&outside_ion, outside_content).unwrap();

    // 扫描根目录 scan_root
    let scan_root = dir.path().join("scan_root");
    fs::create_dir(&scan_root).unwrap();
    let scan_ion = scan_root.join("descript.ion");
    let mut scan_content = Vec::from(TC_HEADER);
    scan_content.extend_from_slice("normal.txt 正常备注\r\n".as_bytes());
    fs::write(&scan_ion, scan_content).unwrap();

    // 在 scan_root 下创建指向 outside_dir 的 Junction
    let link_path = scan_root.join("linked_dir");
    let output = std::process::Command::new("cmd")
        .args([
            "/C",
            "mklink",
            "/J",
            link_path.to_str().unwrap(),
            outside_dir.to_str().unwrap(),
        ])
        .output()
        .expect("failed to execute mklink command");

    assert!(
        output.status.success(),
        "mklink /J failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let mut cmd = Command::cargo_bin("dion").unwrap();
    let assert = cmd
        .current_dir(&scan_root)
        .arg("list")
        .arg("-r")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    let stdout_str = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(stdout_str.contains("normal.txt"));
    // 绝不进入 linked_dir / Junction
    assert!(!stdout_str.contains("secret.txt"));
}

#[test]
fn test_list_diacritics_and_cjk_alignment() {
    let dir = tempdir().unwrap();
    let ion_path = dir.path().join("descript.ion");
    let mut content = Vec::from(TC_HEADER);
    // "café.txt" 长度为 8（拉丁带音符 é 占半角宽度 1）
    content.extend_from_slice("café.txt 拉丁变音符号文件\r\n".as_bytes());
    // "测试.txt" 长度：2个汉字(宽度4) + .txt(宽度4) = 8
    content.extend_from_slice("测试.txt 中文全角文件\r\n".as_bytes());
    fs::write(&ion_path, content).unwrap();

    let mut cmd = Command::cargo_bin("dion").unwrap();
    let assert = cmd
        .current_dir(dir.path())
        .arg("list")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    let stdout_str = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout_str.lines().collect();
    assert_eq!(lines.len(), 2);

    // 两行第一列的显示宽度都是 8，两空格分隔后，第二列对齐起始字节偏移
    let offset_line0 = lines[0].find("拉丁变音符号文件").unwrap();
    // 字符串 "café.txt" 中 'é' 占 2 字节 UTF-8，因此字节偏移为 4 + 2 + 4 + 2 = 12
    assert_eq!(offset_line0, 11);
    // 第二行 "测试.txt" 中每个汉字占 3 字节 UTF-8，字节偏移为 6 + 4 + 2 = 12
    let offset_line1 = lines[1].find("中文全角文件").unwrap();
    assert_eq!(offset_line1, 12);
}

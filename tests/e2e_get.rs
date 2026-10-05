use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

const TC_HEADER: &[u8] = b"\xEF\xBB\xBF\r\n";
const TC_TAIL: &str = "\x04\u{00c2}";

#[test]
fn test_get_single_line_comment() {
    let dir = tempdir().unwrap();
    let ion_path = dir.path().join("descript.ion");
    let mut content = Vec::from(TC_HEADER);
    content.extend_from_slice("hello.txt 这是一个单行备注\r\n".as_bytes());
    fs::write(&ion_path, content).unwrap();

    let mut cmd = Command::cargo_bin("dion").unwrap();
    cmd.current_dir(dir.path())
        .arg("get")
        .arg("hello.txt")
        .assert()
        .success()
        .stdout(predicate::str::diff("这是一个单行备注\r\n"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn test_get_multi_line_comment() {
    let dir = tempdir().unwrap();
    let ion_path = dir.path().join("descript.ion");
    let mut content = Vec::from(TC_HEADER);
    // TC 格式多行：反斜杠转义为 \\，换行转义为 \n，末尾附 \x04\xC3\x82
    let line = format!("multi.txt 第一行\\n第二行\\\\反斜杠{TC_TAIL}\r\n");
    content.extend_from_slice(line.as_bytes());
    fs::write(&ion_path, content).unwrap();

    let mut cmd = Command::cargo_bin("dion").unwrap();
    cmd.current_dir(dir.path())
        .arg("get")
        .arg("multi.txt")
        .assert()
        .success()
        .stdout(predicate::str::diff("第一行\r\n第二行\\反斜杠\r\n"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn test_get_raw_flag() {
    let dir = tempdir().unwrap();
    let ion_path = dir.path().join("descript.ion");
    let mut content = Vec::from(TC_HEADER);
    let line = format!("raw_file.txt 第一行\\n第二行{TC_TAIL}\r\n");
    content.extend_from_slice(line.as_bytes());
    fs::write(&ion_path, content).unwrap();

    let mut cmd = Command::cargo_bin("dion").unwrap();
    let expected_stdout = format!("第一行\\n第二行{TC_TAIL}\r\n");
    cmd.current_dir(dir.path())
        .arg("get")
        .arg("raw_file.txt")
        .arg("--raw")
        .assert()
        .success()
        .stdout(predicate::str::diff(expected_stdout))
        .stderr(predicate::str::is_empty());
}

#[test]
fn test_get_json_flag() {
    let dir = tempdir().unwrap();
    let ion_path = dir.path().join("descript.ion");
    let mut content = Vec::from(TC_HEADER);
    let line = format!("info.txt 测试JSON解码\\n第二行{TC_TAIL}\r\n");
    content.extend_from_slice(line.as_bytes());
    fs::write(&ion_path, content).unwrap();

    let mut cmd = Command::cargo_bin("dion").unwrap();
    let assert = cmd
        .current_dir(dir.path())
        .arg("get")
        .arg("info.txt")
        .arg("--json")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    let output_str = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let val: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    assert_eq!(val["path"], "info.txt");
    assert_eq!(val["name"], "info.txt");
    assert_eq!(val["comment"], "测试JSON解码\r\n第二行");
}

#[test]
fn test_get_raw_and_json_conflict_exit_code_2() {
    let mut cmd = Command::cargo_bin("dion").unwrap();
    cmd.arg("get")
        .arg("file.txt")
        .arg("--raw")
        .arg("--json")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn test_get_aliases_view_and_cat() {
    let dir = tempdir().unwrap();
    let ion_path = dir.path().join("descript.ion");
    let mut content = Vec::from(TC_HEADER);
    content.extend_from_slice("alias.txt 别名测试\r\n".as_bytes());
    fs::write(&ion_path, content).unwrap();

    // view
    let mut cmd_view = Command::cargo_bin("dion").unwrap();
    cmd_view
        .current_dir(dir.path())
        .arg("view")
        .arg("alias.txt")
        .assert()
        .success()
        .stdout(predicate::str::diff("别名测试\r\n"));

    // cat
    let mut cmd_cat = Command::cargo_bin("dion").unwrap();
    cmd_cat
        .current_dir(dir.path())
        .arg("cat")
        .arg("alias.txt")
        .assert()
        .success()
        .stdout(predicate::str::diff("别名测试\r\n"));
}

#[test]
fn test_get_not_found_exit_code_1() {
    let dir = tempdir().unwrap();
    let ion_path = dir.path().join("descript.ion");
    let mut content = Vec::from(TC_HEADER);
    content.extend_from_slice("existing.txt 存在\r\n".as_bytes());
    fs::write(&ion_path, content).unwrap();

    let mut cmd = Command::cargo_bin("dion").unwrap();
    cmd.current_dir(dir.path())
        .arg("get")
        .arg("non_existent.txt")
        .assert()
        .code(1)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_get_not_found_quiet() {
    let dir = tempdir().unwrap();
    let ion_path = dir.path().join("descript.ion");
    let mut content = Vec::from(TC_HEADER);
    content.extend_from_slice("existing.txt 存在\r\n".as_bytes());
    fs::write(&ion_path, content).unwrap();

    // -q
    let mut cmd_q = Command::cargo_bin("dion").unwrap();
    cmd_q
        .current_dir(dir.path())
        .arg("get")
        .arg("non_existent.txt")
        .arg("-q")
        .assert()
        .code(1)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());

    // --quiet
    let mut cmd_quiet = Command::cargo_bin("dion").unwrap();
    cmd_quiet
        .current_dir(dir.path())
        .arg("get")
        .arg("non_existent.txt")
        .arg("--quiet")
        .assert()
        .code(1)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn test_get_missing_descript_ion_exit_code_1() {
    let dir = tempdir().unwrap();

    let mut cmd = Command::cargo_bin("dion").unwrap();
    cmd.current_dir(dir.path())
        .arg("get")
        .arg("anything.txt")
        .assert()
        .code(1)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_get_invalid_header_exit_code_3() {
    let dir = tempdir().unwrap();
    let ion_path = dir.path().join("descript.ion");
    // ANSI 或缺少 BOM 的普通文本
    fs::write(&ion_path, b"test.txt no-bom comment\r\n").unwrap();

    let mut cmd = Command::cargo_bin("dion").unwrap();
    cmd.current_dir(dir.path())
        .arg("get")
        .arg("test.txt")
        .assert()
        .code(3)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("invalid header"));
}

#[test]
fn test_get_case_insensitive_and_quoted_entry() {
    let dir = tempdir().unwrap();
    let ion_path = dir.path().join("descript.ion");
    let mut content = Vec::from(TC_HEADER);
    content.extend_from_slice("\"My Special File.txt\" 空格文件名备注\r\n".as_bytes());
    fs::write(&ion_path, content).unwrap();

    // 以不同大小写输入
    let mut cmd = Command::cargo_bin("dion").unwrap();
    cmd.current_dir(dir.path())
        .arg("get")
        .arg("my special FILE.txt")
        .assert()
        .success()
        .stdout(predicate::str::diff("空格文件名备注\r\n"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn test_get_relative_path_lexical() {
    let dir = tempdir().unwrap();
    let sub = dir.path().join("sub");
    fs::create_dir(&sub).unwrap();

    let ion_path = sub.join("descript.ion");
    let mut content = Vec::from(TC_HEADER);
    content.extend_from_slice("target.txt 子目录备注\r\n".as_bytes());
    fs::write(&ion_path, content).unwrap();

    let mut cmd = Command::cargo_bin("dion").unwrap();
    cmd.current_dir(dir.path())
        .arg("get")
        .arg("sub/target.txt")
        .assert()
        .success()
        .stdout(predicate::str::diff("子目录备注\r\n"));

    // 带 . 和 .. 的冗余路径
    let mut cmd_redundant = Command::cargo_bin("dion").unwrap();
    cmd_redundant
        .current_dir(dir.path())
        .arg("get")
        .arg("./sub/../sub/target.txt")
        .assert()
        .success()
        .stdout(predicate::str::diff("子目录备注\r\n"));
}

#[test]
fn test_get_fixture_real_tc_samples() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixtures_dir = manifest_dir.join(".scratch").join("fixtures");
    if !fixtures_dir.exists() {
        return;
    }

    // 测试根级 fixtures/descript.ion 中的条目
    let mut cmd1 = Command::cargo_bin("dion").unwrap();
    cmd1.current_dir(&fixtures_dir)
        .arg("get")
        .arg("父文件夹1")
        .assert()
        .success()
        .stdout(predicate::str::diff("111测试\r\n"));

    let mut cmd2 = Command::cargo_bin("dion").unwrap();
    cmd2.current_dir(&fixtures_dir)
        .arg("get")
        .arg("父文件夹1 - 副本")
        .assert()
        .success()
        .stdout(predicate::str::diff("222测试\r\n"));

    let mut cmd3 = Command::cargo_bin("dion").unwrap();
    cmd3.current_dir(&fixtures_dir)
        .arg("get")
        .arg("父文件夹2")
        .assert()
        .success()
        .stdout(predicate::str::diff("测试测试\r\n。\r\n"));

    // 测试深层目录 .scratch/fixtures/父文件夹1/88/.env/.env 的多行复杂字符条目
    let env_target = fixtures_dir
        .join("父文件夹1")
        .join("88")
        .join(".env")
        .join(".env");
    let mut cmd4 = Command::cargo_bin("dion").unwrap();
    cmd4.arg("get")
        .arg(env_target.to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::diff("hhhh\r\n😄\r\n\\n\r\n/n\r\n\\\\n\r\n😢\r\n"));
}


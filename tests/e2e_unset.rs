use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_unset_existing_entry_success_and_get_fails() {
    let dir = tempdir().unwrap();
    let target_file = dir.path().join("hello.txt");
    fs::write(&target_file, "content").unwrap();

    // 先通过 dion set 设定备注
    let mut set_cmd = Command::cargo_bin("dion").unwrap();
    set_cmd
        .current_dir(dir.path())
        .arg("set")
        .arg("hello.txt")
        .arg("待删除的备注")
        .assert()
        .success();

    // 再另外设置一个保留文件
    fs::write(dir.path().join("keep.txt"), "").unwrap();
    let mut set_keep_cmd = Command::cargo_bin("dion").unwrap();
    set_keep_cmd
        .current_dir(dir.path())
        .arg("set")
        .arg("keep.txt")
        .arg("保留的备注")
        .assert()
        .success();

    // 执行 dion unset hello.txt
    let mut unset_cmd = Command::cargo_bin("dion").unwrap();
    unset_cmd
        .current_dir(dir.path())
        .arg("unset")
        .arg("hello.txt")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    // 验证后续 dion get hello.txt 返回未找到 (退出码 1)
    let mut get_cmd = Command::cargo_bin("dion").unwrap();
    get_cmd
        .current_dir(dir.path())
        .arg("get")
        .arg("hello.txt")
        .assert()
        .code(1);

    // 验证 keep.txt 仍然存在且内容未受影响
    let mut get_keep_cmd = Command::cargo_bin("dion").unwrap();
    get_keep_cmd
        .current_dir(dir.path())
        .arg("get")
        .arg("keep.txt")
        .assert()
        .success()
        .stdout(predicate::str::diff("保留的备注\r\n"));
}

#[test]
fn test_unset_aliases_rm_and_del() {
    let dir = tempdir().unwrap();

    // 初始化两个条目
    let ion_path = dir.path().join("descript.ion");
    let mut initial = Vec::from(b"\xEF\xBB\xBF\r\n" as &[u8]);
    initial.extend_from_slice(b"file1.txt \xE5\xA4\x87\xE6\xB3\xA81\r\n");
    initial.extend_from_slice(b"file2.txt \xE5\xA4\x87\xE6\xB3\xA82\r\n");
    fs::write(&ion_path, initial).unwrap();

    // 1. 使用别名 rm 删除 file1.txt
    let mut rm_cmd = Command::cargo_bin("dion").unwrap();
    rm_cmd
        .current_dir(dir.path())
        .arg("rm")
        .arg("file1.txt")
        .assert()
        .success();

    // 2. 验证 file1.txt 已被删除
    let mut get1_cmd = Command::cargo_bin("dion").unwrap();
    get1_cmd
        .current_dir(dir.path())
        .arg("get")
        .arg("file1.txt")
        .assert()
        .code(1);

    // 3. 使用别名 del 删除 file2.txt
    let mut del_cmd = Command::cargo_bin("dion").unwrap();
    del_cmd
        .current_dir(dir.path())
        .arg("del")
        .arg("file2.txt")
        .assert()
        .success();

    // 4. 验证 file2.txt 已被删除
    let mut get2_cmd = Command::cargo_bin("dion").unwrap();
    get2_cmd
        .current_dir(dir.path())
        .arg("get")
        .arg("file2.txt")
        .assert()
        .code(1);
}

#[test]
fn test_unset_idempotent_when_entry_or_file_missing() {
    let dir = tempdir().unwrap();

    // 场景 1: descript.ion 根本不存在
    let mut unset_missing_ion = Command::cargo_bin("dion").unwrap();
    unset_missing_ion
        .current_dir(dir.path())
        .arg("unset")
        .arg("nonexistent.txt")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    assert!(!dir.path().join("descript.ion").exists());

    // 场景 2: descript.ion 存在，但没有对应 entry
    let ion_path = dir.path().join("descript.ion");
    let initial_content = b"\xEF\xBB\xBF\r\nother.txt other comment\r\n";
    fs::write(&ion_path, initial_content).unwrap();

    let mut unset_missing_entry = Command::cargo_bin("dion").unwrap();
    unset_missing_entry
        .current_dir(dir.path())
        .arg("unset")
        .arg("nonexistent.txt")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    // 验证文件内容未被篡改
    assert_eq!(fs::read(&ion_path).unwrap(), initial_content);
}

#[test]
fn test_unset_removes_last_entry_deletes_physical_file() {
    let dir = tempdir().unwrap();
    let ion_path = dir.path().join("descript.ion");

    // 1. 只有 1 个条目
    let mut initial = Vec::from(b"\xEF\xBB\xBF\r\n" as &[u8]);
    initial.extend_from_slice(b"only_one.txt single comment\r\n");
    fs::write(&ion_path, initial).unwrap();
    assert!(ion_path.exists());

    let mut unset_cmd = Command::cargo_bin("dion").unwrap();
    unset_cmd
        .current_dir(dir.path())
        .arg("unset")
        .arg("only_one.txt")
        .assert()
        .success();

    // 验证物理 descript.ion 文件已被自动物理删除
    assert!(
        !ion_path.exists(),
        "descript.ion must be removed when all entries are deleted"
    );

    // 2. 有多个条目，删除其中一个条目，文件应保留且顺序保持
    let mut multi = Vec::from(b"\xEF\xBB\xBF\r\n" as &[u8]);
    multi.extend_from_slice(b"first.txt c1\r\n");
    multi.extend_from_slice(b"second.txt c2\r\n");
    multi.extend_from_slice(b"third.txt c3\r\n");
    fs::write(&ion_path, multi).unwrap();

    let mut unset_middle = Command::cargo_bin("dion").unwrap();
    unset_middle
        .current_dir(dir.path())
        .arg("unset")
        .arg("second.txt")
        .assert()
        .success();

    assert!(
        ion_path.exists(),
        "descript.ion must still exist when remaining entries are present"
    );
    let content = fs::read(&ion_path).unwrap();
    let text = String::from_utf8(content[5..].to_vec()).unwrap();
    let lines: Vec<&str> = text
        .lines()
        .map(|l| l.trim_end_matches(['\r', '\n']))
        .filter(|l| !l.is_empty())
        .collect();

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "first.txt c1");
    assert_eq!(lines[1], "third.txt c3");
}

#[test]
fn test_unset_case_insensitive() {
    let dir = tempdir().unwrap();
    let ion_path = dir.path().join("descript.ion");

    let mut initial = Vec::from(b"\xEF\xBB\xBF\r\n" as &[u8]);
    initial.extend_from_slice(b"MixedCase.txt comment\r\n");
    fs::write(&ion_path, initial).unwrap();

    // 用全部大写进行删除
    let mut unset_cmd = Command::cargo_bin("dion").unwrap();
    unset_cmd
        .current_dir(dir.path())
        .arg("unset")
        .arg("MIXEDCASE.TXT")
        .assert()
        .success();

    assert!(!ion_path.exists());
}

#[test]
fn test_set_empty_or_whitespace_comment_triggers_unset() {
    let dir = tempdir().unwrap();
    let ion_path = dir.path().join("descript.ion");

    // 场景 1: 单条目，通过 set "" 移除并清理文件
    fs::write(dir.path().join("target1.txt"), "").unwrap();
    let mut initial = Vec::from(b"\xEF\xBB\xBF\r\n" as &[u8]);
    initial.extend_from_slice(b"target1.txt existing comment\r\n");
    fs::write(&ion_path, initial).unwrap();

    let mut set_empty = Command::cargo_bin("dion").unwrap();
    set_empty
        .current_dir(dir.path())
        .arg("set")
        .arg("target1.txt")
        .arg("")
        .assert()
        .success();

    assert!(
        !ion_path.exists(),
        "set with empty string should unset and clean up empty descript.ion"
    );

    // 场景 2: 多条目，通过 set "   \t  " 移除指定条目，保留其他条目
    fs::write(dir.path().join("remove_me.txt"), "").unwrap();
    let mut multi = Vec::from(b"\xEF\xBB\xBF\r\n" as &[u8]);
    multi.extend_from_slice(b"stay.txt stay comment\r\n");
    multi.extend_from_slice(b"remove_me.txt delete comment\r\n");
    fs::write(&ion_path, multi).unwrap();

    let mut set_spaces = Command::cargo_bin("dion").unwrap();
    set_spaces
        .current_dir(dir.path())
        .arg("set")
        .arg("remove_me.txt")
        .arg("   \t  ")
        .assert()
        .success();

    assert!(ion_path.exists());
    let mut get_stay = Command::cargo_bin("dion").unwrap();
    get_stay
        .current_dir(dir.path())
        .arg("get")
        .arg("stay.txt")
        .assert()
        .success()
        .stdout(predicate::str::diff("stay comment\r\n"));

    let mut get_removed = Command::cargo_bin("dion").unwrap();
    get_removed
        .current_dir(dir.path())
        .arg("get")
        .arg("remove_me.txt")
        .assert()
        .code(1);
}

#[test]
fn test_unset_malformed_file_rejected_exit_code_3() {
    let dir = tempdir().unwrap();
    let ion_path = dir.path().join("descript.ion");
    let corrupt_content = b"Invalid header without TC BOM\r\nfile.txt comment\r\n";
    fs::write(&ion_path, corrupt_content).unwrap();

    let mut unset_cmd = Command::cargo_bin("dion").unwrap();
    unset_cmd
        .current_dir(dir.path())
        .arg("unset")
        .arg("file.txt")
        .assert()
        .code(3)
        .stderr(predicate::str::contains("invalid header"));

    // 验证原损坏文件未被破坏或删除
    assert_eq!(fs::read(&ion_path).unwrap(), corrupt_content);
}

#[test]
fn test_unset_relative_path_lexical() {
    let dir = tempdir().unwrap();
    let sub = dir.path().join("sub_dir");
    fs::create_dir(&sub).unwrap();

    let ion_path = sub.join("descript.ion");
    let mut initial = Vec::from(b"\xEF\xBB\xBF\r\n" as &[u8]);
    initial.extend_from_slice(b"item.txt comment\r\n");
    fs::write(&ion_path, initial).unwrap();

    let mut unset_cmd = Command::cargo_bin("dion").unwrap();
    unset_cmd
        .current_dir(dir.path())
        .arg("unset")
        .arg("sub_dir/item.txt")
        .assert()
        .success();

    assert!(!ion_path.exists());
}

#[test]
fn test_unset_missing_arg_exit_code_2() {
    let mut cmd = Command::cargo_bin("dion").unwrap();
    cmd.arg("unset").assert().code(2);
}

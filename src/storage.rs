//! 存储层与原子持久化模块。
//!
//! 提供 Windows 原生隐藏属性保留、原子替换写入（ReplaceFileW/MoveFileExW 回退）以及文件删除逻辑。

use std::fs;
use std::io::Write;
use std::path::Path;

use crate::error::DionError;

#[cfg(windows)]
mod win_fs {
    use std::ffi::c_void;
    use std::io;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use std::ptr;

    pub const FILE_ATTRIBUTE_HIDDEN: u32 = 0x0000_0002;
    pub const INVALID_FILE_ATTRIBUTES: u32 = 0xFFFF_FFFF;
    pub const MOVEFILE_REPLACE_EXISTING: u32 = 0x0000_0001;
    pub const MOVEFILE_WRITE_THROUGH: u32 = 0x0000_0008;

    extern "system" {
        fn ReplaceFileW(
            lpReplacedFileName: *const u16,
            lpReplacementFileName: *const u16,
            lpBackupFileName: *const u16,
            dwReplaceFlags: u32,
            lpExclude: *mut c_void,
            lpReserved: *mut c_void,
        ) -> i32;

        fn MoveFileExW(
            lpExistingFileName: *const u16,
            lpNewFileName: *const u16,
            dwFlags: u32,
        ) -> i32;

        fn GetFileAttributesW(lpFileName: *const u16) -> u32;

        fn SetFileAttributesW(lpFileName: *const u16, dwFileAttributes: u32) -> i32;
    }

    fn to_wide(path: &Path) -> Vec<u16> {
        path.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    unsafe fn move_file_atomic(tmp_wide: &[u16], target_wide: &[u16]) -> io::Result<()> {
        // SAFETY: 调用者保证 tmp_wide 和 target_wide 是以空字符结尾的合法宽字符指针切片。
        let moved = unsafe {
            MoveFileExW(
                tmp_wide.as_ptr(),
                target_wide.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        };
        if moved == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    pub fn replace_atomic(tmp_path: &Path, target_path: &Path) -> io::Result<()> {
        let tmp_wide = to_wide(tmp_path);
        let target_wide = to_wide(target_path);

        if target_path.exists() {
            // SAFETY: `target_wide` 和 `tmp_wide` 由 `to_wide` 构造，确保以 0 结尾并在当前作用域存活；
            // 备份文件与保留参数传空指针，符合 Win32 API 约定。
            let replaced = unsafe {
                ReplaceFileW(
                    target_wide.as_ptr(),
                    tmp_wide.as_ptr(),
                    ptr::null(),
                    0,
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            };
            if replaced == 0 {
                // 如果 ReplaceFileW 失败（如文件被特定进程占用或平台限制），降级使用 MoveFileExW
                // SAFETY: 传入相同的有效以 null 结尾宽字符数组。
                unsafe { move_file_atomic(&tmp_wide, &target_wide)? };
            }
        } else {
            // SAFETY: 目标文件不存在时，直接原子重命名移位。
            unsafe { move_file_atomic(&tmp_wide, &target_wide)? };
        }

        // 确保目标文件维持 FILE_ATTRIBUTE_HIDDEN
        // SAFETY: `target_wide` 是合法的以 0 结尾宽字符路径指针。
        let current_attrs = unsafe { GetFileAttributesW(target_wide.as_ptr()) };
        let new_attrs = if current_attrs == INVALID_FILE_ATTRIBUTES {
            FILE_ATTRIBUTE_HIDDEN
        } else {
            current_attrs | FILE_ATTRIBUTE_HIDDEN
        };
        // SAFETY: `target_wide` 是合法的以 0 结尾宽字符路径指针，新属性参数为掩码组合。
        unsafe {
            SetFileAttributesW(target_wide.as_ptr(), new_attrs);
        }

        Ok(())
    }
}

/// 原子性写入 `descript.ion` 文件并保持 Windows 隐藏文件属性。
///
/// # Errors
///
/// - 如果临时文件创建、数据刷盘或原子替换失败，返回 [`DionError::Io`]。
pub fn atomic_write_ion(parent_dir: &Path, content: &[u8]) -> Result<(), DionError> {
    let pid = std::process::id();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());

    let tmp_name = format!("descript.ion.{pid}_{timestamp}.tmp");
    let tmp_path = parent_dir.join(tmp_name);
    let target_path = parent_dir.join("descript.ion");

    // 辅助闭包/Guard 确保出错时清理临时文件
    let write_result = (|| -> Result<(), DionError> {
        let mut opts = fs::OpenOptions::new();
        opts.write(true).create_new(true);

        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            opts.custom_flags(win_fs::FILE_ATTRIBUTE_HIDDEN);
        }

        let mut file = opts.open(&tmp_path).map_err(DionError::Io)?;
        file.write_all(content).map_err(DionError::Io)?;
        file.sync_all().map_err(DionError::Io)?;
        drop(file);

        #[cfg(windows)]
        {
            win_fs::replace_atomic(&tmp_path, &target_path).map_err(DionError::Io)?;
        }

        #[cfg(not(windows))]
        {
            fs::rename(&tmp_path, &target_path).map_err(DionError::Io)?;
        }

        Ok(())
    })();

    if write_result.is_err() && tmp_path.exists() {
        let _ = fs::remove_file(&tmp_path);
    }

    write_result
}

/// 删除指定目录下的 `descript.ion` 文件。
///
/// 若文件本就不存在，则视为成功（幂等性）。
///
/// # Errors
///
/// - 如果底层文件删除遭遇权限或非 `NotFound` 类 I/O 错误，返回 [`DionError::Io`]。
pub fn remove_ion(parent_dir: &Path) -> Result<(), DionError> {
    let target_path = parent_dir.join("descript.ion");
    match fs::remove_file(&target_path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(DionError::Io(e)),
    }
}

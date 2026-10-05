use crate::error::DionError;
use std::fs;
use std::io::Write;
use std::path::Path;

#[cfg(windows)]
mod win_fs {
    use std::ffi::c_void;
    use std::io;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use std::ptr;

    pub const FILE_ATTRIBUTE_HIDDEN: u32 = 0x00000002;
    pub const INVALID_FILE_ATTRIBUTES: u32 = 0xFFFFFFFF;
    pub const MOVEFILE_REPLACE_EXISTING: u32 = 0x00000001;
    pub const MOVEFILE_WRITE_THROUGH: u32 = 0x00000008;

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
        let moved = MoveFileExW(
            tmp_wide.as_ptr(),
            target_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        );
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
                // 如果 ReplaceFileW 失败，降级使用 MoveFileExW
                unsafe { move_file_atomic(&tmp_wide, &target_wide)? };
            }
        } else {
            unsafe { move_file_atomic(&tmp_wide, &target_wide)? };
        }

        // 确保目标文件维持 FILE_ATTRIBUTE_HIDDEN
        let current_attrs = unsafe { GetFileAttributesW(target_wide.as_ptr()) };
        let new_attrs = if current_attrs == INVALID_FILE_ATTRIBUTES {
            FILE_ATTRIBUTE_HIDDEN
        } else {
            current_attrs | FILE_ATTRIBUTE_HIDDEN
        };
        unsafe {
            SetFileAttributesW(target_wide.as_ptr(), new_attrs);
        }

        Ok(())
    }
}

pub fn atomic_write_ion(parent_dir: &Path, content: &[u8]) -> Result<(), DionError> {
    let pid = std::process::id();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

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

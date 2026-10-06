//! 领域错误类型定义模块。
//!
//! 基于 `thiserror` 统一定义错误变体并映射标准化 CLI 退出码。

use thiserror::Error;

/// Dion 工具的统一领域错误枚举。
#[derive(Error, Debug)]
pub enum DionError {
    /// 找不到目标文件或目录的备注（退出码 1）
    #[error("comment not found for '{0}'")]
    NotFound(String),

    /// `descript.ion` 文件缺少 Total Commander UTF-8 规范头 `0xEFBBBF0D0A`
    #[error("invalid header: descript.ion must begin with 0xEFBBBF0D0A")]
    InvalidHeader,

    /// `descript.ion` 包含非法 UTF-8 字节序列
    #[error("invalid UTF-8 content in descript.ion")]
    InvalidUtf8,

    /// 行格式异常（如引号未闭合或大小写冲突条目）
    #[error("malformed descript.ion line: {0}")]
    MalformedEntry(String),

    /// 底层文件读写或进程启动发生系统 I/O 错误
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// 路径无效（如空白路径或越界路径）
    #[error("invalid path: {0}")]
    InvalidPath(String),

    /// 命令行参数冲突、缺失或环境不满足（如非 TTY 尝试交互式编辑，退出码 2）
    #[error("{0}")]
    Usage(String),

    /// 单行存储长度超过 Total Commander 规范上限 4096 字节
    #[error("line exceeds 4096 bytes limit for '{0}': {1} bytes")]
    LineTooLong(String, usize),
}

impl DionError {
    /// 获取错误对应的 CLI 进程退出状态码。
    ///
    /// - `1`: 未找到目标条目备注（NotFound）
    /// - `2`: 命令行用法错误或参数冲突（Usage）
    /// - `3`: 数据格式损坏、I/O 故障或其他运行时错误
    #[must_use]
    pub fn exit_code(&self) -> i32 {
        match self {
            DionError::NotFound(_) => 1,
            DionError::Usage(_) => 2,
            _ => 3,
        }
    }
}

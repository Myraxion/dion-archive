use thiserror::Error;

#[derive(Error, Debug)]
pub enum DionError {
    #[error("comment not found for '{0}'")]
    NotFound(String),

    #[error("invalid header: descript.ion must begin with 0xEFBBBF0D0A")]
    InvalidHeader,

    #[error("invalid UTF-8 content in descript.ion")]
    InvalidUtf8,

    #[error("malformed descript.ion line: {0}")]
    MalformedEntry(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid path: {0}")]
    InvalidPath(String),

    #[error("line exceeds 4096 bytes limit for '{0}': {1} bytes")]
    LineTooLong(String, usize),
}

impl DionError {
    pub fn exit_code(&self) -> i32 {
        match self {
            DionError::NotFound(_) => 1,
            _ => 3,
        }
    }
}

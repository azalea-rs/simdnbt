use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReadError {
    #[error("Invalid root type {0}")]
    InvalidRootType(u8),
    #[error("Unknown tag id {0}")]
    UnknownTagId(u8),
    #[error("Unexpected end of data")]
    UnexpectedEof,
    #[error("Tried to read NBT tag with too high complexity, depth > 512")]
    MaxDepthExceeded,
}

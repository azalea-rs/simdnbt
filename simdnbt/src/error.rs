use thiserror::Error;

use crate::common::MAX_DEPTH;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("Invalid root type {0}")]
    InvalidRootType(u8),
    #[error("Unknown tag id {0}")]
    UnknownTagId(u8),
    #[error("Unexpected end of data")]
    UnexpectedEof,
    #[error("Tried to read NBT tag with too high complexity, depth > {MAX_DEPTH}")]
    MaxDepthExceeded,
}

#[derive(Error, Debug)]
pub enum DeserializeError {
    #[error("Missing field")]
    MissingField,
    #[error("Mismatched type for {0}")]
    MismatchedFieldType(String),
    #[error("Unknown field {0:?}")]
    UnknownField(String),
}

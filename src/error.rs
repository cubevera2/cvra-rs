use std::{fmt, io};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    BadSignature,
    UnsupportedVersion(u16),
    ReservedBits(&'static str),
    UnexpectedEof,
    VarintOverflow,
    InvalidUtf8,
    InvalidStringIndex(u32),
    InvalidVisibility,
    UnknownShape(u8),
    UnknownMaterial(u8),
    TrailingData(&'static str),
    LimitExceeded(&'static str),
    TooLarge,
    CompressionUnavailable,
    Compression(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::BadSignature => write!(f, "not a .cvra file (bad signature)"),
            Error::UnsupportedVersion(v) => write!(f, "unsupported format version {v}"),
            Error::ReservedBits(what) => write!(f, "reserved bits set in {what}"),
            Error::UnexpectedEof => write!(f, "unexpected end of data"),
            Error::VarintOverflow => write!(f, "varint does not fit in 32 bits"),
            Error::InvalidUtf8 => write!(f, "string is not valid utf-8"),
            Error::InvalidStringIndex(i) => write!(f, "string table index {i} out of range"),
            Error::InvalidVisibility => write!(f, "invalid visibility value"),
            Error::UnknownShape(tag) => write!(f, "unknown shape tag {tag}"),
            Error::UnknownMaterial(tag) => write!(f, "unknown material tag {tag}"),
            Error::TrailingData(what) => write!(f, "unexpected trailing bytes in {what}"),
            Error::LimitExceeded(what) => write!(f, "limit exceeded: {what}"),
            Error::TooLarge => write!(f, "value too large for the format"),
            Error::CompressionUnavailable => {
                write!(f, "compression requires the `zstd` feature")
            }
            Error::Compression(e) => write!(f, "compression error: {e}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Compression(e) => Some(e),

            _ => None,
        }
    }
}

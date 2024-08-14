/// Errors which can occur during transport layer operations.
#[derive(thiserror::Error, Debug)]
pub enum TransportError {
    /// The stream has reached the end of the file, there is no more data to be read.
    #[error("End of file reached.")]
    EOF,
    /// Some variable number sent was too large to be decoded.
    #[error("Variable number too large.")]
    VarNumTooLarge,
    /// An I/O error occurred which was unrelated to the processing of the packet.
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    /// An error occurred while trying to parse a UUID.
    #[cfg(feature = "uuid")]
    #[error(transparent)]
    UuidError(#[from] uuid::Error),
    /// An error occurred while trying to decode a UTF-8 string.
    #[error(transparent)]
    Utf8Error(#[from] std::string::FromUtf8Error),
    /// A limit exceeded during decoding or encoding.
    #[error("Limit exceeded while {2}. Expected {0} but received {1}.")]
    LimitExceeded(i32, i32, &'static str),
    /// An error occurred during the serialization or deserialization process from serde_json.
    #[cfg(feature = "serde")]
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
    /// Nbt related errors.
    #[cfg(feature = "nbt")]
    #[error(transparent)]
    NbtError(#[from] NbtError),
}

impl TransportError {
    pub fn limit_exceeded<T>(expected: i32, received: i32, context: &'static str) -> DraxResult<T> {
        Err(Self::LimitExceeded(expected, received, context))
    }
}

/// Result type alias for transport errors.
pub type DraxResult<T> = Result<T, TransportError>;

/// Nbt encoding and decoding errors.
#[cfg(feature = "nbt")]
#[derive(thiserror::Error, Debug)]
pub enum NbtError {
    #[error("NBT tag too complex. Depth surpassed 512.")]
    ComplexTag,
    #[error("Invalid tag bit {0}. Could not load tag.")]
    InvalidTagBit(u8),
    #[error("Nbt tag too big. Expected {0} but received {1}.")]
    TagTooBig(u64, u64),
    #[error("Nbt accounter overflowed. Could not read nbt.")]
    AccounterOverflow,
    #[error("Cesu8 decoding error. {0}")]
    Cesu8DecodingError(#[from] cesu8::Cesu8DecodingError),
}

#[cfg(feature = "nbt")]
impl NbtError {
    pub fn complex_tag<T>() -> DraxResult<T> {
        Err(Self::ComplexTag.into())
    }

    pub fn invalid_tag_bit<T>(bit: u8) -> DraxResult<T> {
        Err(Self::InvalidTagBit(bit).into())
    }

    pub fn tag_too_big<T>(expected: u64, received: u64) -> DraxResult<T> {
        Err(Self::TagTooBig(expected, received).into())
    }

    pub fn accounter_overflow<T>() -> DraxResult<T> {
        Err(Self::AccounterOverflow.into())
    }
}

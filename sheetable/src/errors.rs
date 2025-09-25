use std::{error::Error, fmt};

/// Struct-level error type used by [`crate::traits::Sheetable`] and [`crate::traits::SheetableReadOnly`].
///
/// Per-field conversion errors are wrapped in this type to keep public APIs
/// ergonomic and consistent.
#[derive(Debug)]
pub enum SheetError {
    /// A required value was missing at the provided index in a row.
    MissingValue { index: usize },

    /// A field failed to encode.
    Encode {
        field: &'static str,
        source: Box<dyn Error + Send + Sync>,
    },

    /// A field failed to decode.
    Decode {
        field: &'static str,
        source: Box<dyn Error + Send + Sync>,
    },

    /// Generic message.
    Message(String),
}

impl SheetError {
    /// Construct a `MissingValue` error.
    pub fn missing(index: usize) -> Self {
        SheetError::MissingValue { index }
    }

    /// Wrap an encode error for the named field.
    pub fn encode<E>(field: &'static str, err: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        SheetError::Encode {
            field,
            source: Box::new(err),
        }
    }

    /// Wrap a decode error for the named field.
    pub fn decode<E>(field: &'static str, err: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        SheetError::Decode {
            field,
            source: Box::new(err),
        }
    }
}

impl fmt::Display for SheetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SheetError::MissingValue { index } => write!(f, "missing value at index {index}"),
            SheetError::Encode { field, source } => {
                write!(f, "encode error in `{field}`: {source}")
            }
            SheetError::Decode { field, source } => {
                write!(f, "decode error in `{field}`: {source}")
            }
            SheetError::Message(s) => write!(f, "{s}"),
        }
    }
}
impl Error for SheetError {}

pub type Result<T> = std::result::Result<T, SheetError>;

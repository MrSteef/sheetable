use serde_json::Value;

use crate::errors::SheetError;

/// Mapping for a row struct with writable columns.
///
/// Implementations should encode only writable columns in [`Sheetable::to_values`], and
/// decode both writable and read-only columns in [`Sheetable::from_values`]. The return
/// type of [`Sheetable::from_values`] is the concrete **hydrated** struct for the row:
/// - If a row has no calculated fields, `Hydrated` is typically `Self`.
/// - If a row has calculated fields, `Hydrated` is usually the same struct with
///   its read-only generic filled in (e.g. `User<UserDetails>`).
pub trait Sheetable: Sized {
    /// The read-only bundle type (use `()` if none).
    type ReadOnly: SheetableReadOnly;

    /// The concrete hydrated type produced by [`Sheetable::from_values`].
    type Hydrated;

    /// Encode writable columns to cells. Calculated fields are ignored.
    fn to_values(&self) -> Result<Vec<Value>, SheetError>;

    /// Decode a fully hydrated row from cells.
    fn from_values(values: &[Value]) -> Result<Self::Hydrated, SheetError>;
}

/// Bundle of read-only (calculated) columns decoded from the sheet.
///
/// Types that represent calculated columns implement this trait. They are never
/// written back to the sheet.
pub trait SheetableReadOnly: Sized {
    /// Decode the read-only bundle from a slice of cells.
    fn from_values(values: &[Value]) -> Result<Self, SheetError>;
}

impl SheetableReadOnly for () {
    fn from_values(_: &[Value]) -> Result<Self, SheetError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{DecodeCell, EncodeCell};

    use super::*;
    use serde_json::Value;

    #[derive(Debug, Clone, PartialEq)]
    struct Dummy(u8);

    impl EncodeCell for Dummy {
        type Error = std::convert::Infallible;
        fn encode_cell(&self) -> Result<Value, Self::Error> {
            Ok(Value::Number(self.0.into()))
        }
    }

    impl DecodeCell for Dummy {
        type Error = std::convert::Infallible;
        fn decode_cell(v: &Value) -> Result<Self, Self::Error> {
            if let Value::Number(n) = v {
                Ok(Dummy(n.as_u64().unwrap() as u8))
            } else {
                Ok(Dummy(0))
            }
        }
    }

    #[derive(Debug, Clone)]
    struct TestRow<T: EncodeCell + DecodeCell> {
        a: T,
        b: T,
    }

    impl<T: EncodeCell + DecodeCell> Sheetable for TestRow<T> {
        type ReadOnly = ();
        type Hydrated = TestRow<T>;

        fn to_values(&self) -> Result<Vec<Value>, SheetError> {
            Ok(vec![
                self.a
                    .encode_cell()
                    .map_err(|e| SheetError::encode("a", e))?,
                self.b
                    .encode_cell()
                    .map_err(|e| SheetError::encode("b", e))?,
            ])
        }

        fn from_values(values: &[Value]) -> Result<Self::Hydrated, SheetError> {
            if values.len() < 2 {
                return Err(SheetError::missing(1));
            }
            Ok(TestRow {
                a: T::decode_cell(&values[0]).map_err(|e| SheetError::decode("a", e))?,
                b: T::decode_cell(&values[1]).map_err(|e| SheetError::decode("b", e))?,
            })
        }
    }

    #[test]
    fn sheetable_roundtrip_with_dummy() {
        let row = TestRow {
            a: Dummy(1),
            b: Dummy(2),
        };
        let values = row.to_values().unwrap();
        assert_eq!(values.len(), 2);

        let hydrated = TestRow::<Dummy>::from_values(&values).unwrap();
        assert_eq!(hydrated.a.0, 1);
        assert_eq!(hydrated.b.0, 2);
    }
}

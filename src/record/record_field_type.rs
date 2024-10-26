use std::borrow::Cow;

use crate::{utils, value::Value};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum RecordFieldType {
    Null,
    I8,
    I16,
    I24,
    I32,
    I48,
    I64,
    Float,
    Zero,
    One,
    String(usize),
    Blob(usize),
}

impl RecordFieldType {
    pub fn parse(discriminant: i64) -> anyhow::Result<RecordFieldType> {
        match discriminant {
            0 => Ok(RecordFieldType::Null),
            1 => Ok(RecordFieldType::I8),
            2 => Ok(RecordFieldType::I16),
            3 => Ok(RecordFieldType::I24),
            4 => Ok(RecordFieldType::I32),
            5 => Ok(RecordFieldType::I48),
            6 => Ok(RecordFieldType::I64),
            7 => Ok(RecordFieldType::Float),
            8 => Ok(RecordFieldType::Zero),
            9 => Ok(RecordFieldType::One),
            n if n >= 12 && n % 2 == 0 => {
                let size = ((n - 12) / 2) as usize;
                Ok(RecordFieldType::Blob(size))
            }
            n if n >= 13 && n % 2 == 1 => {
                let size = ((n - 13) / 2) as usize;
                Ok(RecordFieldType::String(size))
            }
            n => Err(anyhow::anyhow!("unsupported field type: {}", n)),
        }
    }

    pub fn size(self) -> usize {
        match self {
            Self::Null => 0,
            Self::I8 => 1,
            Self::I16 => 2,
            Self::I24 => 3,
            Self::I32 => 4,
            Self::I48 => 6,
            Self::I64 => 8,
            Self::Float => 8,
            Self::Zero => 0,
            Self::One => 0,
            Self::Blob(s) => s,
            Self::String(s) => s,
        }
    }

    pub fn value(self, payload: &[u8], offset: usize) -> Option<Value> {
        match self {
            Self::Null => Some(Value::Null),
            Self::I8 => Some(Value::Int(utils::read_i8_at(payload, offset))),
            Self::I16 => Some(Value::Int(utils::read_i16_at(payload, offset))),
            Self::I24 => Some(Value::Int(utils::read_i24_at(payload, offset))),
            Self::I32 => Some(Value::Int(utils::read_i32_at(payload, offset))),
            Self::I48 => Some(Value::Int(utils::read_i48_at(payload, offset))),
            Self::I64 => Some(Value::Int(utils::read_i64_at(payload, offset))),
            Self::Float => Some(Value::Float(utils::read_f64_at(payload, offset))),
            Self::Blob(length) => {
                let value = &payload[offset..offset + length];
                Some(Value::Blob(Cow::Borrowed(value)))
            }
            Self::String(length) => {
                let value =
                    std::str::from_utf8(&payload[offset..offset + length]).expect("invalid utf8");
                Some(Value::String(Cow::Borrowed(value)))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn record_field_type_value_tests() -> () {
        assert_eq!(Some(Value::Null), RecordFieldType::Null.value(&[], 0));
        assert_eq!(Some(Value::Int(1)), RecordFieldType::I8.value(&[1], 0));
        assert_eq!(
            Some(Value::Int(257)),
            RecordFieldType::I16.value(&[1, 1], 0)
        );
        assert_eq!(
            Some(Value::Int(65793)),
            RecordFieldType::I24.value(&[1, 1, 1], 0)
        );
        assert_eq!(
            Some(Value::Int(16843009)),
            RecordFieldType::I32.value(&[1, 1, 1, 1], 0)
        );
        assert_eq!(
            Some(Value::Int(1099511627777)),
            RecordFieldType::I48.value(&[1, 0, 0, 0, 0, 1], 0)
        );
        assert_eq!(
            Some(Value::Int(72057594037927936)),
            RecordFieldType::I64.value(&[1, 0, 0, 0, 0, 0, 0, 0], 0)
        );
    }

    #[test]
    fn parse_record_field_type_tests() -> () {
        assert_eq!(RecordFieldType::Null, RecordFieldType::parse(0).unwrap());
        assert_eq!(RecordFieldType::I8, RecordFieldType::parse(1).unwrap());
        assert_eq!(RecordFieldType::I16, RecordFieldType::parse(2).unwrap());
        assert_eq!(RecordFieldType::I24, RecordFieldType::parse(3).unwrap());
        assert_eq!(RecordFieldType::I32, RecordFieldType::parse(4).unwrap());
        assert_eq!(RecordFieldType::I48, RecordFieldType::parse(5).unwrap());
        assert_eq!(RecordFieldType::I64, RecordFieldType::parse(6).unwrap());
        assert_eq!(RecordFieldType::Float, RecordFieldType::parse(7).unwrap());
        assert_eq!(RecordFieldType::Zero, RecordFieldType::parse(8).unwrap());
        assert_eq!(RecordFieldType::One, RecordFieldType::parse(9).unwrap());
        assert!(RecordFieldType::parse(10).is_err());
        assert!(RecordFieldType::parse(11).is_err());
        assert_eq!(
            RecordFieldType::Blob(0),
            RecordFieldType::parse(12).unwrap()
        );
        assert_eq!(
            RecordFieldType::Blob(1),
            RecordFieldType::parse(14).unwrap()
        );
        assert_eq!(
            RecordFieldType::String(0),
            RecordFieldType::parse(13).unwrap()
        );
        assert_eq!(
            RecordFieldType::String(1),
            RecordFieldType::parse(15).unwrap()
        );
    }

    #[test]
    fn record_field_type_size_tests() -> () {
        assert_eq!(RecordFieldType::Null.size(), 0);
        assert_eq!(RecordFieldType::I8.size(), 1);
        assert_eq!(RecordFieldType::I16.size(), 2);
        assert_eq!(RecordFieldType::I24.size(), 3);
        assert_eq!(RecordFieldType::I32.size(), 4);
        assert_eq!(RecordFieldType::I48.size(), 6);
        assert_eq!(RecordFieldType::I64.size(), 8);
        assert_eq!(RecordFieldType::Float.size(), 8);
        assert_eq!(RecordFieldType::Zero.size(), 0);
        assert_eq!(RecordFieldType::One.size(), 0);
        assert_eq!(RecordFieldType::Blob(0).size(), 0);
        assert_eq!(RecordFieldType::String(0).size(), 0);
    }
}
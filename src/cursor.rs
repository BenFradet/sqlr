use std::borrow::Cow;

use crate::{page::Page, pager::Pager, utils, value::Value};

#[derive(Debug)]
pub struct Cursor<'p> {
    header: RecordHeader,
    pager: &'p mut Pager,
    page_index: usize,
    page_cell: usize,
}

impl<'p> Cursor<'p> {
    pub fn new(
        header: RecordHeader,
        pager: &'p mut Pager,
        page_index: usize,
        page_cell: usize,
    ) -> Self {
        Self {
            header,
            pager,
            page_index,
            page_cell,
        }
    }

    pub fn field(&mut self, n: usize) -> Option<Value> {
        let record_field = self.header.fields.get(n)?;

        let payload = match self.pager.read_page(self.page_index) {
            Ok(Page::TableLeaf(leaf)) => &leaf.cells[self.page_cell].payload,
            _ => return None,
        };

        record_field.field_type.value(payload, record_field.offset)
    }
}

fn read_i8_at(input: &[u8], offset: usize) -> i64 {
    input[offset] as i64
}

fn read_i16_at(input: &[u8], offset: usize) -> i64 {
    i16::from_be_bytes(input[offset..offset + 2].try_into().unwrap()) as i64
}

fn read_i24_at(input: &[u8], offset: usize) -> i64 {
    (i32::from_be_bytes(input[offset..offset + 3].try_into().unwrap()) & 0x00FFFFFF) as i64
}

fn read_i32_at(input: &[u8], offset: usize) -> i64 {
    i32::from_be_bytes(input[offset..offset + 4].try_into().unwrap()) as i64
}

fn read_i48_at(input: &[u8], offset: usize) -> i64 {
    i64::from_be_bytes(input[offset..offset + 6].try_into().unwrap()) & 0x0000FFFFFFFFFFFF
}

fn read_i64_at(input: &[u8], offset: usize) -> i64 {
    i64::from_be_bytes(input[offset..offset + 8].try_into().unwrap())
}

fn read_f64_at(input: &[u8], offset: usize) -> f64 {
    f64::from_be_bytes(input[offset..offset + 8].try_into().unwrap())
}

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
    fn parse(discriminant: i64) -> anyhow::Result<RecordFieldType> {
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

    fn size(self) -> usize {
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

    fn value(self, payload: &[u8], offset: usize) -> Option<Value> {
        match self {
            Self::Null => Some(Value::Null),
            Self::I8 => Some(Value::Int(read_i8_at(payload, offset))),
            Self::I16 => Some(Value::Int(read_i16_at(payload, offset))),
            Self::I24 => Some(Value::Int(read_i24_at(payload, offset))),
            Self::I32 => Some(Value::Int(read_i32_at(payload, offset))),
            Self::I48 => Some(Value::Int(read_i48_at(payload, offset))),
            Self::I64 => Some(Value::Int(read_i64_at(payload, offset))),
            Self::Float => Some(Value::Float(read_f64_at(payload, offset))),
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

#[derive(Debug, Clone)]
pub struct RecordField {
    pub offset: usize,
    pub field_type: RecordFieldType,
}

#[derive(Debug, Clone)]
pub struct RecordHeader {
    pub fields: Vec<RecordField>,
}

impl RecordHeader {
    pub fn parse(mut buffer: &[u8]) -> anyhow::Result<RecordHeader> {
        let (varint_size, header_length) = utils::read_varint_at(buffer, 0);
        buffer = &buffer[varint_size as usize..header_length as usize];

        let mut fields = Vec::new();
        let mut current_offset = header_length as usize;

        while !buffer.is_empty() {
            let (discriminant_size, discriminant) = utils::read_varint_at(buffer, 0);
            buffer = &buffer[discriminant_size as usize..];

            let field_type = RecordFieldType::parse(discriminant)?;
            let field_size = field_type.size();

            fields.push(RecordField {
                offset: current_offset,
                field_type,
            });

            current_offset += field_size;
        }

        Ok(RecordHeader { fields })
    }
}

#[cfg(test)]
mod test {
    use super::*;

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

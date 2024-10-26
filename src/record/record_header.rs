use crate::utils;

use super::{record_field::RecordField, record_field_type::RecordFieldType};

#[derive(Debug, Clone, PartialEq)]
pub struct RecordHeader {
    pub fields: Vec<RecordField>,
}

impl RecordHeader {
    pub fn parse(mut buffer: &[u8]) -> anyhow::Result<RecordHeader> {
        let (varint_size, header_length) = utils::read_varint_at(buffer, 0);

        if header_length < varint_size as i64 || buffer.len() < header_length as usize {
            anyhow::bail!("header length too large")
        }

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
    fn record_header_parse_tests() -> () {
        assert!(RecordHeader::parse(&vec![0b10000001, 0b01111111]).is_err());
        // 10 record field type unsupported
        assert!(RecordHeader::parse(&[2, 10]).is_err());
        assert_eq!(
            RecordHeader {
                fields: vec![RecordField {
                    offset: 2,
                    field_type: RecordFieldType::Zero
                }]
            },
            RecordHeader::parse(&[2, 8]).unwrap()
        );
    }
}
use crate::pager;

#[derive(Debug, Copy, Clone)]
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
    String,
    Blob,
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

fn parse_record_header(mut buffer: &[u8]) -> anyhow::Result<RecordHeader> {
    let (varint_size, header_length) = pager::read_varint_at(buffer, 0);
    buffer = &buffer[varint_size as usize..header_length as usize];

    let mut fields = Vec::new();
    let mut current_offset = header_length as usize;

    while !buffer.is_empty() {
        let (discriminant_size, discriminant) = pager::read_varint_at(buffer, 0);
        buffer = &buffer[discriminant_size as usize..];

        let (field_type, field_size) = match discriminant {
            0 => (RecordFieldType::Null, 0),
            1 => (RecordFieldType::I8, 1),
            2 => (RecordFieldType::I16, 2),
            3 => (RecordFieldType::I24, 3),
            4 => (RecordFieldType::I32, 4),
            5 => (RecordFieldType::I48, 6),
            6 => (RecordFieldType::I64, 8),
            7 => (RecordFieldType::Float, 8),
            8 => (RecordFieldType::Zero, 0),
            9 => (RecordFieldType::One, 0),
            n if n >= 12 && n % 2 == 0 => {
                let size = ((n - 12) / 2) as usize;
                (RecordFieldType::String, size)
            },
            n if n >= 13 && n %2 == 1 => {
                let size = ((n - 13) / 2) as usize;
                (RecordFieldType::Blob, size)
            },
            n => anyhow::bail!("unsupported field type: {}", n),
        };

        fields.push(RecordField {
            offset: current_offset,
            field_type,
        });

        current_offset += field_size;
    }

    Ok(RecordHeader { fields })
}
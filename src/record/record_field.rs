use super::record_field_type::RecordFieldType;

#[derive(Debug, Clone, PartialEq)]
pub struct RecordField {
    pub offset: usize,
    pub field_type: RecordFieldType,
}
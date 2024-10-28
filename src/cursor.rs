use crate::{record::record_header::RecordHeader, value::Value};

#[derive(Debug)]
pub struct Cursor {
    pub header: RecordHeader,
    pub payload: Vec<u8>
}

impl Cursor {
    pub fn new(
        header: RecordHeader,
        payload: Vec<u8>,
    ) -> Self {
        Self { header, payload }
    }

    pub fn field(&self, n: usize) -> Option<Value> {
        let record_field = self.header.fields.get(n)?;
        record_field.field_type.value(&self.payload, record_field.offset)
    }
}

#[cfg(test)]
mod test {
    use crate::{db::DbHeader, page::{cell::Cell, page::HEADER_SIZE, pager::{FilePager, Pager}}, record::record_header::RecordHeader};

    use super::*;

    use std::{borrow::Cow, io::Read};

    #[test]
    fn cursor_field_tests() -> () {
        let mut file = std::fs::File::open("test.db").unwrap();
        let mut header_buffer = [0; HEADER_SIZE];
        file.read_exact(&mut header_buffer).unwrap();
        let db_header = DbHeader::parse(&header_buffer).unwrap();
        let mut pager = FilePager::new(file, db_header.page_size as usize);
        let page_nr = 1;
        let page = pager.read_page(page_nr).unwrap();
        let cell = page.cells.get(0).unwrap();
        let (header, payload) = match cell {
            Cell::TableLeaf(c) => (RecordHeader::parse(&c.payload).unwrap(), c.payload.clone()),
            Cell::TableInterior(c) => panic!("not a leaf: {:?}", c),
        };
        let cursor = Cursor::new(header, payload);
        assert_eq!(Some(Value::String(Cow::from("table"))), cursor.field(0));
        assert_eq!(Some(Value::String(Cow::from("tbl1"))), cursor.field(1));
    }
}

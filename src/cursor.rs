use crate::{page::cell::Cell, pager::Pager, record::record_header::RecordHeader, value::Value};

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
            Ok(page) => match &page.cells[self.page_cell] {
                Cell::TableLeaf(leaf) => &leaf.payload,
                _ => return None,
            },
            _ => return None,
        };

        record_field.field_type.value(&payload, record_field.offset)
    }
}

#[cfg(test)]
mod test {
    use crate::{db::DbHeader, page::page::HEADER_SIZE, record::record_header::RecordHeader};

    use super::*;

    use std::{borrow::Cow, io::Read};

    #[test]
    fn cursor_field_tests() -> () {
        let mut file = std::fs::File::open("test.db").unwrap();
        let mut header_buffer = [0; HEADER_SIZE];
        file.read_exact(&mut header_buffer).unwrap();
        let db_header = DbHeader::parse(&header_buffer).unwrap();
        let mut pager = Pager::new(file, db_header.page_size as usize);
        let page_nr = 1;
        let page = pager.read_page(page_nr).unwrap();
        let cell = page.cells.get(0).unwrap();
        let header = match cell {
            Cell::TableLeaf(c) => RecordHeader::parse(&c.payload).unwrap(),
            Cell::TableInterior(c) => panic!("not a leaf: {:?}", c),
        };
        let mut cursor = Cursor::new(header, &mut pager, 1, 0);
        assert_eq!(Some(Value::String(Cow::from("table"))), cursor.field(0));
        assert_eq!(Some(Value::String(Cow::from("tbl1"))), cursor.field(1));
    }
}

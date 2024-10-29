use crate::{cursor::Cursor, page::{cell::Cell, pager::Pager, positioned_page::PositionedPage}, record::record_header::RecordHeader};

#[derive(Debug)]
enum ScannerElem {
    PagePointer(u32),
    Cursor(Cursor),
}

pub struct Scanner<'p> {
    pager: &'p mut dyn Pager,
    initial_page_num: usize,
    page_stack: Vec<PositionedPage>,
}

impl<'p> Scanner<'p> {
    pub fn new(pager: &'p mut dyn Pager, initial_page_num: usize) -> Scanner<'p> {
        Scanner {
            pager,
            initial_page_num,
            page_stack: Vec::new(),
        }
    }

    pub fn next_record(&mut self) -> anyhow::Result<Option<Cursor>> {
        loop {
            match self.next_elem() {
                Ok(Some(ScannerElem::Cursor(cursor))) => return Ok(Some(cursor)),
                Ok(Some(ScannerElem::PagePointer(page_pointer))) => {
                    let new_page = self.pager.read_page(page_pointer as usize)?.clone();
                    self.page_stack.push(PositionedPage{
                        page: new_page,
                        cell_num: 0,
                    });
                },
                Ok(None) if self.page_stack.len() > 1 => {
                    self.page_stack.pop();
                },
                Ok(None) => return Ok(None),
                Err(e) => return Err(e),
            }
        }
    }

    fn next_elem(&mut self) -> anyhow::Result<Option<ScannerElem>> {
        let Some(pos_page) = self.current_page()? else {
            return Ok(None);
        };

        if let Some(page_pointer) = pos_page.next_page_pointer() {
            return Ok(Some(ScannerElem::PagePointer(page_pointer)));
        }

        let Some(cell) = pos_page.next_cell() else {
            return Ok(None);
        };

        match cell {
            Cell::TableLeaf(leaf) => {
                let header = RecordHeader::parse(&leaf.payload)?;
                // TODO: remove clone
                Ok(Some(ScannerElem::Cursor(Cursor::new(header, leaf.payload.clone()))))
            },
            Cell::TableInterior(interior) => Ok(Some(ScannerElem::PagePointer(interior.left_child_page))),
        }
    }

    fn current_page(&mut self) -> anyhow::Result<Option<&mut PositionedPage>> {
        if self.page_stack.is_empty() {
            let page = match self.pager.read_page(self.initial_page_num) {
                Ok(page) => page.clone(),
                Err(e) => return Err(e),
            };

            self.page_stack.push(PositionedPage { page, cell_num: 0 });
        }
        Ok(self.page_stack.last_mut())
    }
}

#[cfg(test)]
mod test {
    use std::collections::{hash_map::Entry, HashMap};

    use crate::{page::{cell::{TableInteriorCell, TableLeafCell}, page::Page, page_header::PageHeader, pager::FilePager}, record::{record_field::RecordField, record_field_type::RecordFieldType}};

    use super::*;

    #[test]
    fn next_elem_tests() -> () {
        // points to the rightmost pointer
        let int_header = PageHeader::TableInteriorPageHeader {
            first_freeblock: 0,
            cell_count: 2,
            cell_content_offset: 0,
            fragmented_bytes_count: 0,
            rightmost_pointer: 12,
        };
        let empty_int_page = empty_page(int_header);
        let mut pager = MockPager { reader: |_| Ok(empty_int_page.clone()), pages: HashMap::new() };
        let mut scanner = Scanner::new(&mut pager, 0);
        let next_elem = scanner.next_elem();
        assert!(next_elem.is_ok());
        match next_elem.unwrap() {
            Some(ScannerElem::PagePointer(p)) => assert_eq!(12, p),
            _ => panic!("not a page pointer")
        }

        // no next cell
        let leaf_header = PageHeader::TableLeafPageHeader {
            first_freeblock: 0,
            cell_count: 2,
            cell_content_offset: 0,
            fragmented_bytes_count: 0,
        };
        let empty_leaf_page = empty_page(leaf_header);
        let mut pager = MockPager { reader: |_| Ok(empty_leaf_page.clone()), pages: HashMap::new() };
        let mut scanner = Scanner::new(&mut pager, 0);
        let next_elem = scanner.next_elem();
        assert!(next_elem.is_ok());
        match next_elem.unwrap() {
            None => (),
            _ => panic!("not none")
        }

        // cell is leaf
        let leaf_page = Page {
            header: leaf_header,
            cell_pointers: vec![0],
            cells: vec![TableLeafCell {
                size: 2,
                row_id: 0,
                payload: vec![2, 8],
            }.into()],
        };
        let mut pager = MockPager { reader: |_| Ok(leaf_page.clone()), pages: HashMap::new() };
        let mut scanner = Scanner::new(&mut pager, 0);
        let next_elem = scanner.next_elem();
        assert!(next_elem.is_ok());
        match next_elem.unwrap() {
            Some(ScannerElem::Cursor(Cursor { header, payload })) => {
                assert_eq!(RecordHeader {
                    fields: vec![RecordField {
                        offset: 2,
                        field_type: RecordFieldType::Zero
                    }],
                }, header);
                assert_eq!(vec![2, 8], payload);
            },
            _ => panic!("not cursor")
        }

        // cell is int
        let leaf_page = Page {
            header: leaf_header,
            cell_pointers: vec![0],
            cells: vec![TableInteriorCell {
                left_child_page: 38,
                key: 0,
            }.into()],
        };
        let mut pager = MockPager { reader: |_| Ok(leaf_page.clone()), pages: HashMap::new() };
        let mut scanner = Scanner::new(&mut pager, 0);
        let next_elem = scanner.next_elem();
        assert!(next_elem.is_ok());
        match next_elem.unwrap() {
            Some(ScannerElem::PagePointer(p)) => assert_eq!(38, p),
            _ => panic!("not cursor")
        }
    }

    fn empty_page(header: PageHeader) -> Page {
        Page {
            header: header,
            cell_pointers: vec![],
            cells: vec![],
        }
    }

    struct MockPager<F> where F: Fn(usize) -> anyhow::Result<Page> {
        reader: F,
        pages: HashMap<usize, Page>,
    }

    impl<F> Pager for MockPager<F> where F: Fn(usize) -> anyhow::Result<Page> {
        fn load_page(&mut self, page_num: usize) -> anyhow::Result<Page> {
            (self.reader)(page_num)
        }

        fn read_page<'a>(&'a mut self, page_num: usize) -> anyhow::Result<&'a Page> {
            if let Entry::Vacant(_) = self.pages.entry(page_num) {
                let page = self.load_page(page_num)?;
                self.pages.insert(page_num, page);
            }
            Ok(self.pages.get(&page_num).unwrap())
        }
    }

    #[test]
    fn current_page_tests() -> () {
        let file = std::fs::File::open("test.db").unwrap();
        let mut pager = FilePager::new(file, 4096);

        let mut scanner1 = Scanner::new(&mut pager, 10);
        assert!(scanner1.current_page().is_err());

        let mut scanner2 = Scanner::new(&mut pager, 0);
        let current_page = scanner2.current_page();
        assert!(current_page.is_ok());
        assert_eq!(Some(PositionedPage {
            page: Page {
                header: crate::page::page_header::PageHeader::TableLeafPageHeader {
                    first_freeblock: 0,
                    cell_count: 0,
                    cell_content_offset: 0,
                    fragmented_bytes_count: 0
                },
                cell_pointers: vec![],
                cells: vec![],
            },
            cell_num: 0,
        }), current_page.unwrap().cloned());
    }
}

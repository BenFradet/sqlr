use crate::{cursor::{parse_record_header, Cursor}, page::Page, pager::Pager};

#[derive(Debug)]
pub struct Scanner<'p> {
    pager: &'p mut Pager,
    page: usize,
    cell: usize,
}

impl<'p> Scanner<'p> {
    pub fn new(pager: &'p mut Pager, page: usize) -> Scanner<'p> {
        Scanner {
            pager,
            page,
            cell: 0,
        }
    }

    pub fn next_record(&mut self) -> Option<anyhow::Result<Cursor>> {
        let page = match self.pager.read_page(self.page) {
            Ok(page) => page,
            Err(e) => return Some(Err(e)),
        };

        match page {
            Page::TableLeaf(leaf) => {
                let cell = leaf.cells.get(self.cell)?;

                let header = match parse_record_header(&cell.payload) {
                    Ok(header) => header,
                    Err(e) => return Some(Err(e)),
                };

                let record = Cursor::new(
                    header,
                    self.pager,
                    self.page,
                    self.cell,
                );

                self.cell += 1;

                Some(Ok(record))
            }
        }
    }
}
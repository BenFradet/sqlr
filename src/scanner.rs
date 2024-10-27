use crate::{cursor::Cursor, page::{cell::Cell, pager::Pager, positioned_page::PositionedPage}, record::record_header::RecordHeader};

#[derive(Debug)]
enum ScannerElem {
    PagePointer(u32),
    Cursor(Cursor),
}

#[derive(Debug)]
pub struct Scanner<'p> {
    pager: &'p mut Pager,
    initial_page: usize,
    page_stack: Vec<PositionedPage>,
}

impl<'p> Scanner<'p> {
    pub fn new(pager: &'p mut Pager, page: usize) -> Scanner<'p> {
        Scanner {
            pager,
            initial_page: page,
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
                Ok(Some(ScannerElem::Cursor(Cursor::new(header, leaf.payload.clone()))))
            },
            Cell::TableInterior(interior) => Ok(Some(ScannerElem::PagePointer(interior.left_child_page))),
        }
    }

    fn current_page(&mut self) -> anyhow::Result<Option<&mut PositionedPage>> {
        if self.page_stack.is_empty() {
            let page = match self.pager.read_page(self.initial_page) {
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
}

use super::{cell::Cell, page::Page, page_header::PageHeader};

#[derive(Debug, Clone, PartialEq)]
pub struct PositionedPage {
    pub page: Page,
    pub cell: usize,
}

impl PositionedPage {
    pub fn next_cell(&mut self) -> Option<&Cell> {
        let cell = self.page.cells.get(self.cell);
        self.cell += 1;
        cell
    }

    pub fn next_page_pointer(&mut self) -> Option<u32> {
        match self.page.header {
            PageHeader::TableInteriorPageHeader { .. } =>
                if self.page.cells.len() == self.cell {
                    self.cell += 1;
                    self.page.header.rightmost_pointer()
                } else {
                    None
                }
            _ => None
        }
    }
}
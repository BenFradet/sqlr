use super::{cell::Cell, page::Page, page_header::PageHeader};

#[derive(Debug, Clone, PartialEq)]
pub struct PositionedPage {
    pub page: Page,
    pub cell_num: usize,
}

impl PositionedPage {
    pub fn next_cell(&mut self) -> Option<&Cell> {
        self.page.cells.get(self.cell_num).inspect(|_| self.cell_num += 1)
    }

    pub fn next_page_pointer(&mut self) -> Option<u32> {
        match self.page.header {
            PageHeader::TableInteriorPageHeader { .. } =>
                if self.cell_num >= self.page.cells.len() {
                    self.cell_num = self.page.cells.len() + 1;
                    self.page.header.rightmost_pointer()
                } else {
                    None
                }
            _ => None
        }
    }
}

#[cfg(test)]
mod test {
    use crate::page::cell::{TableInteriorCell, TableLeafCell};

    use super::*;

    #[test]
    fn next_cell_leaf_tests() -> () {
        let c1: Cell = TableLeafCell {
            size: 2,
            row_id: 12,
            payload: vec![127, 128],
        }.into();
        let c2: Cell = TableLeafCell {
            size: 3,
            row_id: 13,
            payload: vec![127, 128, 129],
        }.into();
        let page = Page {
            header: PageHeader::TableLeafPageHeader {
                first_freeblock: 0,
                cell_count: 2,
                cell_content_offset: 0,
                fragmented_bytes_count: 0,
            },
            cell_pointers: vec![1, 10, 12],
            cells: vec![c1.clone(), c2.clone()],
        };
        let mut p_page = PositionedPage { page: page, cell_num: 0 };
        let res1 = p_page.next_cell().cloned();
        let res2 = p_page.next_cell().cloned();
        let res3 = p_page.next_cell();
        assert_eq!(Some(c1), res1);
        assert_eq!(Some(c2), res2);
        assert_eq!(None, res3);
        assert_eq!(2, p_page.cell_num);
    }

    #[test]
    fn next_cell_interior_tests() -> () {
        let c1: Cell = TableInteriorCell {
            left_child_page: 1,
            key: 12,
        }.into();
        let c2: Cell = TableInteriorCell {
            left_child_page: 2,
            key: 13,
        }.into();
        let page = Page {
            header: PageHeader::TableInteriorPageHeader {
                first_freeblock: 0,
                cell_count: 2,
                cell_content_offset: 0,
                fragmented_bytes_count: 0,
                rightmost_pointer: 12,
            },
            cell_pointers: vec![1, 10, 12],
            cells: vec![c1.clone(), c2.clone()],
        };
        let mut p_page = PositionedPage { page: page, cell_num: 0 };
        let res1 = p_page.next_cell().cloned();
        let res2 = p_page.next_cell().cloned();
        let res3 = p_page.next_cell();
        assert_eq!(Some(c1), res1);
        assert_eq!(Some(c2), res2);
        assert_eq!(None, res3);
        assert_eq!(2, p_page.cell_num);
    }
}
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
    fn next_page_pointer_tests() -> () {
        let leaf_header = PageHeader::TableLeafPageHeader {
            first_freeblock: 0,
            cell_count: 2,
            cell_content_offset: 0,
            fragmented_bytes_count: 0,
        };
        let leaf_page = Page {
            header: leaf_header,
            cell_pointers: vec![],
            cells: vec![],
        };
        let mut leaf_p_page = PositionedPage { page: leaf_page, cell_num: 0 };
        assert_eq!(None, leaf_p_page.next_page_pointer());
        assert_eq!(0, leaf_p_page.cell_num);

        let rightmost_pointer = 12;
        let c1: Cell = TableInteriorCell {
            left_child_page: 1,
            key: 12,
        }.into();
        let int_header = PageHeader::TableInteriorPageHeader {
            first_freeblock: 0,
            cell_count: 2,
            cell_content_offset: 0,
            fragmented_bytes_count: 0,
            rightmost_pointer: rightmost_pointer,
        };
        let int_page = Page {
            header: int_header,
            cell_pointers: vec![],
            cells: vec![c1],
        };
        let mut int_p_page = PositionedPage { page: int_page.clone(), cell_num: 1 };
        assert_eq!(Some(rightmost_pointer), int_p_page.next_page_pointer());
        assert_eq!(2, int_p_page.cell_num);
        let mut int_p_page_2 = PositionedPage { page: int_page, cell_num: 0 };
        assert_eq!(None, int_p_page_2.next_page_pointer());
        assert_eq!(0, int_p_page_2.cell_num);
    }

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
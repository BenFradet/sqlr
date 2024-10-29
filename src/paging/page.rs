use crate::utils;

use super::{
    cell::{Cell, TableInteriorCell, TableLeafCell},
    page_header::PageHeader,
};

pub const HEADER_SIZE: usize = 100;

// https://www.sqlite.org/fileformat.html

#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    pub header: PageHeader,
    pub cell_pointers: Vec<u16>,
    pub cells: Vec<Cell>,
}

impl Page {
    pub fn parse(buffer: &[u8], page_num: usize) -> anyhow::Result<Page> {
        let ptr_offset = if page_num == 1 { HEADER_SIZE as u16 } else { 0 };

        let content_buffer = &buffer[ptr_offset as usize..];
        let header = PageHeader::parse(content_buffer)?;

        let cell_pointers = Self::parse_cell_pointers(
            &content_buffer[header.byte_size()..],
            header.cell_count() as usize,
            ptr_offset,
        );

        let cell_parsing_fn = match header {
            PageHeader::TableInteriorPageHeader { .. } => TableInteriorCell::parse,
            PageHeader::TableLeafPageHeader { .. } => TableLeafCell::parse,
        };

        let cells = Self::parse_cells(content_buffer, &cell_pointers, cell_parsing_fn)?;

        Ok(Self {
            header,
            cell_pointers,
            cells,
        })
    }

    fn parse_cells(
        buffer: &[u8],
        cell_pointers: &[u16],
        parse_fn: impl Fn(&[u8]) -> anyhow::Result<Cell>,
    ) -> anyhow::Result<Vec<Cell>> {
        cell_pointers
            .iter()
            .map(|&ptr| parse_fn(&buffer[ptr as usize..]))
            .collect()
    }

    // turns [u8] into [u16]
    fn parse_cell_pointers(buffer: &[u8], n: usize, ptr_offset: u16) -> Vec<u16> {
        let mut pointers = Vec::with_capacity(n);
        for i in 0..n {
            let offset = 2 * i;
            if offset + 2 <= buffer.len() {
                pointers.push(utils::read_be_word_at(buffer, offset).1 - ptr_offset);
            } else {
                break;
            }
        }
        pointers
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_interior_cells_tests() -> () {
        let buffer = [
            1, 0, 0, 0, 127, // interior cell 1
            0, 0, 0, 1, 12, // interior cell 2
        ];
        let cell_pointers = [0, 5];
        let parse_fn = TableInteriorCell::parse;
        let res = Page::parse_cells(&buffer, &cell_pointers, parse_fn);
        assert!(res.is_ok());
        let expected: Vec<Cell> = vec![
            TableInteriorCell {
                left_child_page: 16777216,
                key: 127,
            }
            .into(),
            TableInteriorCell {
                left_child_page: 1,
                key: 12,
            }
            .into(),
        ];
        assert_eq!(expected, res.unwrap());
    }

    #[test]
    fn parse_leaf_cells_tests() -> () {
        let buffer = [
            2, 1, 127, 128, // leaf cell 1
            1, 2, 127, // leaf cell 2
        ];
        let cell_pointers = [0, 3];
        let parse_fn = TableLeafCell::parse;
        let res = Page::parse_cells(&buffer, &cell_pointers, parse_fn);
        assert!(res.is_ok());
        let expected: Vec<Cell> = vec![
            TableLeafCell {
                size: 2,
                row_id: 1,
                payload: vec![127, 128],
            }
            .into(),
            TableLeafCell {
                size: 1,
                row_id: 2,
                payload: vec![127],
            }
            .into(),
        ];
        assert_eq!(expected, res.unwrap());
    }

    #[test]
    fn parse_table_interior_page_tests() -> () {
        assert!(Page::parse(&[12], 0).is_err());
        let buffer = [
            // page header w/ 2 as cell count
            5, 0, 12, 0, 2, 0, 0, 0, 0, 0, 0, 21, // cell pointer
            0, 16, 0, 21, // interior cell (left_child_page, key)
            0, 0, 0, 1, 10, 1, 0, 0, 0, 129, 0,
        ];
        let res = Page::parse(&buffer, 0);
        assert!(res.is_ok());
        let expected = Page {
            header: PageHeader::TableInteriorPageHeader {
                first_freeblock: 12,
                cell_count: 2,
                cell_content_offset: 65536,
                fragmented_bytes_count: 0,
                rightmost_pointer: 21,
            },
            cell_pointers: vec![16, 21],
            cells: vec![
                TableInteriorCell {
                    left_child_page: 1,
                    key: 10,
                }
                .into(),
                TableInteriorCell {
                    left_child_page: 16777216,
                    key: 128,
                }
                .into(),
            ],
        };
        assert_eq!(expected, res.unwrap());
    }

    #[test]
    fn parse_table_leaf_page_tests() -> () {
        assert!(Page::parse(&[12], 0).is_err());
        let buffer = [
            // page header w/ 1 as cell count
            13, 0, 12, 0, 1, 0, 0, 0, // cell pointer
            0, 10, // leaf cell (size, row id, payload)
            10, 2, 127,
        ];
        let res = Page::parse(&buffer, 0);
        assert!(res.is_ok());
        let expected = Page {
            header: PageHeader::TableLeafPageHeader {
                first_freeblock: 12,
                cell_count: 1,
                cell_content_offset: 65536,
                fragmented_bytes_count: 0,
            },
            cell_pointers: vec![10],
            cells: vec![TableLeafCell {
                size: 10,
                row_id: 2,
                payload: vec![127],
            }
            .into()],
        };
        assert_eq!(expected, res.unwrap());
    }

    #[test]
    fn parse_cell_pointers_test() -> () {
        assert_eq!(vec![65535], Page::parse_cell_pointers(&[255, 255], 1, 0));
        assert_eq!(vec![65535], Page::parse_cell_pointers(&[255, 255], 2, 0));
        assert_eq!(
            vec![65435],
            Page::parse_cell_pointers(&[255, 255], 1, HEADER_SIZE as u16)
        );
    }
}

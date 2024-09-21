use crate::utils;

pub const PAGE_MAX_SIZE: u32 = 65536;
pub const HEADER_SIZE: usize = 100;

// https://www.sqlite.org/fileformat.html

#[derive(Debug, Clone, PartialEq)]
pub enum Page {
    TableLeaf(TableLeafPage),
}

impl Page {
    pub fn parse(buffer: &[u8], page_num: usize) -> anyhow::Result<Page> {
        let ptr_offset = if page_num == 1 { HEADER_SIZE as u16 } else { 0 };

        match PageType::parse(buffer) {
            Ok(PageType::TableLeaf) => {
                TableLeafPage::parse(buffer, ptr_offset).map(Page::TableLeaf)
            }
            _ => Err(anyhow::anyhow!("unknown page type: {}", buffer[0])),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableLeafPage {
    pub header: PageHeader,
    pub cell_pointers: Vec<u16>,
    pub cells: Vec<TableLeafCell>,
}

impl TableLeafPage {
    const PAGE_LEAF_HEADER_SIZE: usize = 8;
    fn parse(buffer: &[u8], ptr_offset: u16) -> anyhow::Result<TableLeafPage> {
        let header = PageHeader::parse(buffer)?;

        let content_buffer = &buffer[Self::PAGE_LEAF_HEADER_SIZE..];
        let cell_pointers =
            Self::parse_cell_pointers(content_buffer, header.cell_count() as usize, ptr_offset);

        let cells = cell_pointers
            .iter()
            .map(|&ptr| TableLeafCell::parse(&buffer[ptr as usize..]))
            .collect::<anyhow::Result<Vec<TableLeafCell>>>()?;

        Ok(TableLeafPage {
            header,
            cell_pointers,
            cells,
        })
    }

    // turns [u8] into [u16]
    fn parse_cell_pointers(buffer: &[u8], n: usize, ptr_offset: u16) -> Vec<u16> {
        let mut pointers = Vec::with_capacity(n);
        for i in 0..n {
            let offset = 2 * i;
            if offset + 2 <= buffer.len() {
                pointers.push(utils::read_be_word_at(buffer, offset) - ptr_offset);
            } else {
                break;
            }
        }
        pointers
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PageHeader {
    TableLeafPageHeader {
        first_freeblock: u16,
        cell_count: u16,
        cell_content_offset: u32,
        fragmented_bytes_count: u8,
    },
    TableInteriorPageHeader {
        first_freeblock: u16,
        cell_count: u16,
        cell_content_offset: u32,
        fragmented_bytes_count: u8,
        rightmost_pointer: u32,
    },
}

impl PageHeader {
    const PAGE_FIRST_FREEBLOCK_OFFSET: usize = 1;
    const PAGE_CELL_COUNT_OFFSET: usize = 3;
    const PAGE_CELL_CONTENT_OFFSET: usize = 5;
    const PAGE_FRAGMENTED_BYTES_COUNT_OFFSET: usize = 7;
    const PAGE_RIGHTMOST_POINTER_OFFSET: usize = 8;

    const PAGE_HEADER_SIZE_LEAF: usize = 8;
    const PAGE_HEADER_SIZE_INTERIOR: usize = 12;

    // 0th byte => 13 for a table btree leaf
    // word at 1 byte offset => first free block offset in the page, 0 if no free block
    // word at 3 byte offset => cell count in the page
    // word at 5 byte offset => offset of the first cell
    // 7th byte => number of fragmented free bytes in the page (probably used for vaccuuming?)
    // 2 words at 8 byte offset => right most pointer in interior b-tree pages
    pub fn parse(buffer: &[u8]) -> anyhow::Result<PageHeader> {
        if buffer.len() < 7 {
            return Err(anyhow::anyhow!("page header must be at least 7-byte long"));
        }

        let page_type = PageType::parse(buffer)?;

        let first_freeblock = utils::read_be_word_at(buffer, Self::PAGE_FIRST_FREEBLOCK_OFFSET);
        let cell_count = utils::read_be_word_at(buffer, Self::PAGE_CELL_COUNT_OFFSET);
        let cell_content_offset =
            match utils::read_be_word_at(buffer, Self::PAGE_CELL_CONTENT_OFFSET) {
                0 => PAGE_MAX_SIZE,
                n => n as u32,
            };
        let fragmented_bytes_count = buffer[Self::PAGE_FRAGMENTED_BYTES_COUNT_OFFSET];

        let header = if page_type == PageType::TableInterior {
            PageHeader::TableInteriorPageHeader {
                first_freeblock,
                cell_count,
                cell_content_offset,
                fragmented_bytes_count,
                rightmost_pointer: utils::read_be_double_word_at(
                    buffer,
                    Self::PAGE_RIGHTMOST_POINTER_OFFSET,
                ),
            }
        } else {
            PageHeader::TableLeafPageHeader {
                first_freeblock,
                cell_count,
                cell_content_offset,
                fragmented_bytes_count,
            }
        };

        Ok(header)
    }

    pub fn first_freeblock(&self) -> u16 {
        match *self {
            PageHeader::TableInteriorPageHeader {
                first_freeblock, ..
            }
            | PageHeader::TableLeafPageHeader {
                first_freeblock, ..
            } => first_freeblock,
        }
    }

    pub fn cell_count(&self) -> u16 {
        match *self {
            PageHeader::TableInteriorPageHeader { cell_count, .. }
            | PageHeader::TableLeafPageHeader { cell_count, .. } => cell_count,
        }
    }

    pub fn cell_content_offset(&self) -> u32 {
        match *self {
            PageHeader::TableInteriorPageHeader {
                cell_content_offset,
                ..
            }
            | PageHeader::TableLeafPageHeader {
                cell_content_offset,
                ..
            } => cell_content_offset,
        }
    }

    pub fn fragmented_bytes_count(&self) -> u8 {
        match *self {
            PageHeader::TableInteriorPageHeader {
                fragmented_bytes_count,
                ..
            }
            | PageHeader::TableLeafPageHeader {
                fragmented_bytes_count,
                ..
            } => fragmented_bytes_count,
        }
    }

    pub fn rightmost_pointer(&self) -> Option<u32> {
        match *self {
            PageHeader::TableInteriorPageHeader {
                rightmost_pointer, ..
            } => Some(rightmost_pointer),
            PageHeader::TableLeafPageHeader { .. } => None,
        }
    }

    pub fn byte_size(&self) -> usize {
        match self {
            PageHeader::TableInteriorPageHeader { .. } => Self::PAGE_HEADER_SIZE_INTERIOR,
            PageHeader::TableLeafPageHeader { .. } => Self::PAGE_HEADER_SIZE_LEAF,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PageType {
    TableLeaf,
    TableInterior,
}

impl PageType {
    const PAGE_LEAF_TABLE_ID: u8 = 13;
    const PAGE_INTERIOR_TABLE_ID: u8 = 5;

    // 2: interior index b-tree page
    // 5: interior table b-tree page
    // 10: leaf index b-tree page
    // 13: leaf table b-tree page
    fn parse(buffer: &[u8]) -> anyhow::Result<PageType> {
        match buffer[0] {
            Self::PAGE_LEAF_TABLE_ID => Ok(PageType::TableLeaf),
            Self::PAGE_INTERIOR_TABLE_ID => Ok(PageType::TableInterior),
            _ => Err(anyhow::anyhow!("unknown page type: {}", buffer[0])),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableLeafCell {
    pub size: i64,
    pub row_id: i64,
    pub payload: Vec<u8>,
}

impl TableLeafCell {
    // format is:
    // - size of the payload: varint
    // - row id: varint
    // - payload
    fn parse(mut buffer: &[u8]) -> anyhow::Result<TableLeafCell> {
        let (n, size) = utils::read_varint_at(buffer, 0);
        buffer = &buffer[n as usize..];

        let (n, row_id) = utils::read_varint_at(buffer, 0);
        buffer = &buffer[n as usize..];

        let su = size as usize;
        let payload = if su <= buffer.len() {
            buffer[..su].to_vec()
        } else {
            buffer.to_vec()
        };

        Ok(TableLeafCell {
            size,
            row_id,
            payload,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_page_tests() -> () {
        assert!(Page::parse(&[12], 0).is_err());
        let buffer = [
            // page header w/ 1 as cell count
            13, 0, 12, 0, 1, 0, 0, 0, // cell pointer
            0, 10, // leaf cell (size, row id, payload)
            10, 2, 127,
        ];
        let res = Page::parse(&buffer, 0);
        assert!(res.is_ok());
        let expected = Page::TableLeaf(TableLeafPage {
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
            }],
        });
        assert_eq!(expected, res.unwrap());
    }

    #[test]
    fn parse_table_leaf_page_tests() -> () {
        assert!(TableLeafPage::parse(&[12], 0).is_err());
        let buffer = [
            // page header w/ 1 as cell count
            13, 0, 12, 0, 1, 0, 0, 0, // cell pointer
            0, 10, // leaf cell (size, row id, payload)
            10, 2, 127,
        ];
        let res = TableLeafPage::parse(&buffer, 0);
        assert!(res.is_ok());
        let expected = TableLeafPage {
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
            }],
        };
        assert_eq!(expected, res.unwrap());
    }

    #[test]
    fn parse_cell_pointers_test() -> () {
        assert_eq!(
            vec![65535],
            TableLeafPage::parse_cell_pointers(&[255, 255], 1, 0)
        );
        assert_eq!(
            vec![65535],
            TableLeafPage::parse_cell_pointers(&[255, 255], 2, 0)
        );
        assert_eq!(
            vec![65435],
            TableLeafPage::parse_cell_pointers(&[255, 255], 1, HEADER_SIZE as u16)
        );
    }

    #[test]
    fn parse_page_header_tests() -> () {
        // first byte must be 13 for a table b-tree leaf
        assert!(PageHeader::parse(&[12]).is_err());
        assert!(PageHeader::parse(&[12, 0, 12, 0, 11, 0, 10, 0]).is_err());
        let res = PageHeader::parse(&[13, 0, 12, 0, 11, 0, 0, 0]);
        assert!(res.is_ok());
        let expected = PageHeader::TableLeafPageHeader {
            first_freeblock: 12,
            cell_count: 11,
            cell_content_offset: 65536,
            fragmented_bytes_count: 0,
        };
        assert_eq!(expected, res.unwrap());
    }

    #[test]
    fn parse_table_leaf_cell_tests() -> () {
        let size = 10;
        let row_id = 2;
        let payload = 127;
        let input = [size, row_id, payload];
        let res = TableLeafCell::parse(&input);
        let expected = TableLeafCell {
            size: size as i64,
            row_id: row_id as i64,
            payload: vec![payload],
        };
        assert!(res.is_ok());
        assert_eq!(expected, res.unwrap());
    }

    #[test]
    fn parse_page_type_tests() -> () {
        assert!(PageType::parse(&[12]).is_err());
        let res = PageType::parse(&[13]);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), PageType::TableLeaf);
    }
}

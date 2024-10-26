use crate::utils;

use super::page_type::PageType;

pub const PAGE_MAX_SIZE: u32 = 65536;

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
        // points to the root of the subtree that contains keys > any keys in the page's cells
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

        let first_freeblock = utils::read_be_word_at(buffer, Self::PAGE_FIRST_FREEBLOCK_OFFSET).1;
        let cell_count = utils::read_be_word_at(buffer, Self::PAGE_CELL_COUNT_OFFSET).1;
        let cell_content_offset =
            match utils::read_be_word_at(buffer, Self::PAGE_CELL_CONTENT_OFFSET).1 {
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
                ).1,
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn page_header_accessor_tests() -> () {
        let leaf = PageHeader::TableLeafPageHeader {
            first_freeblock: 12,
            cell_count: 1,
            cell_content_offset: 65536,
            fragmented_bytes_count: 0,
        };
        let interior = PageHeader::TableInteriorPageHeader {
            first_freeblock: 12,
            cell_count: 1,
            cell_content_offset: 65536,
            fragmented_bytes_count: 0,
            rightmost_pointer: 12,
        };
        assert_eq!(leaf.cell_content_offset(), 65536);
        assert_eq!(interior.cell_content_offset(), 65536);
        assert_eq!(leaf.cell_count(), 1);
        assert_eq!(interior.cell_count(), 1);
        assert_eq!(leaf.first_freeblock(), 12);
        assert_eq!(interior.first_freeblock(), 12);
        assert_eq!(leaf.fragmented_bytes_count(), 0);
        assert_eq!(interior.fragmented_bytes_count(), 0);
        assert_eq!(leaf.rightmost_pointer(), None);
        assert_eq!(interior.rightmost_pointer(), Some(12));
        assert_eq!(leaf.byte_size(), 8);
        assert_eq!(interior.byte_size(), 12);
    }

    #[test]
    fn parse_page_header_tests() -> () {
        // first byte must be 13 for a table b-tree leaf
        assert!(PageHeader::parse(&[12]).is_err());
        assert!(PageHeader::parse(&[12, 0, 12, 0, 11, 0, 10, 0]).is_err());
        assert_eq!(
            PageHeader::TableLeafPageHeader {
                first_freeblock: 12,
                cell_count: 11,
                cell_content_offset: 65536,
                fragmented_bytes_count: 0,
            },
            PageHeader::parse(&[13, 0, 12, 0, 11, 0, 0, 0]).unwrap()
        );
        // interior
        assert_eq!(
            PageHeader::TableInteriorPageHeader {
                first_freeblock: 12,
                cell_count: 11,
                cell_content_offset: 65536,
                fragmented_bytes_count: 0,
                rightmost_pointer: 16909060,
            },
            PageHeader::parse(&[5, 0, 12, 0, 11, 0, 0, 0, 1, 2, 3, 4]).unwrap(),
        );
        assert_eq!(
            PageHeader::TableInteriorPageHeader {
                first_freeblock: 12,
                cell_count: 11,
                cell_content_offset: 65536,
                fragmented_bytes_count: 0,
                rightmost_pointer: 0,
            },
            PageHeader::parse(&[5, 0, 12, 0, 11, 0, 0, 0]).unwrap(),
        );
    }
}
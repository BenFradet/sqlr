use std::{collections::HashMap, io::{Read, Seek}};

use anyhow::Context;

use crate::page::*;

pub const HEADER_SIZE: usize = 100;

// https://www.sqlite.org/fileformat.html

#[derive(Debug, Clone)]
pub struct Pager<I: Read + Seek = std::fs::File> {
    input: I,
    page_size: usize,
    pages: HashMap<usize, Page>,
}

impl<I: Read + Seek> Pager<I> {
    pub fn new(input: I, page_size: usize) -> Self {
        Self {
            input,
            page_size,
            pages: HashMap::new(),
        }
    }

    pub fn read_page(&mut self, n: usize) -> anyhow::Result<&Page> {
        if self.pages.contains_key(&n) {
            Ok(self.pages.get(&n).unwrap())
        } else {
            let page = self.load_page(n)?;
            self.pages.insert(n, page);
            Ok(self.pages.get(&n).unwrap())
        }
    }

    fn load_page(&mut self, n: usize) -> anyhow::Result<Page> {
        let offset = HEADER_SIZE + n.saturating_sub(1) * self.page_size;

        self.input
            .seek(std::io::SeekFrom::Start(offset as u64))
            .context("seek to page start")?;

        let mut buffer = vec![0; self.page_size];
        self.input.read_exact(&mut buffer).context("read page")?;

        parse_page(&buffer, n)
    }
}

const HEADER_PREFIX: &[u8] = b"SQLite format 3\0";
const HEADER_PAGE_SIZE_OFFSET: usize = 16;
const PAGE_MAX_SIZE: u32 = 65536;
pub fn parse_header(buffer: &[u8]) -> anyhow::Result<DbHeader> {
    if !buffer.starts_with(HEADER_PREFIX) {
        let prefix = String::from_utf8_lossy(&buffer[..HEADER_PREFIX.len()]);
        Err(anyhow::anyhow!("invalid header prefix: {prefix}"))
    } else {
        let page_size_raw = read_be_word_at(buffer, HEADER_PAGE_SIZE_OFFSET);
        let page_size = match page_size_raw {
            1 => Ok(PAGE_MAX_SIZE),
            n if n.is_power_of_two() => Ok(n as u32),
            _ => Err(anyhow::anyhow!("page size is not a power of 2: {}", page_size_raw)),
        };
        page_size.map(|page_size| DbHeader { page_size })
    }
}

fn parse_page(buffer: &[u8], page_num: usize) -> anyhow::Result<Page> {
    let ptr_offset = if page_num == 1 { HEADER_SIZE as u16 } else { 0 };

    match parse_page_type(buffer) {
        Ok(PageType::TableLeaf) => parse_table_leaf_page(buffer, ptr_offset),
        _ => Err(anyhow::anyhow!("unknown page type: {}", buffer[0])),
    }
}

const PAGE_LEAF_HEADER_SIZE: usize = 8;
fn parse_table_leaf_page(buffer: &[u8], ptr_offset: u16) -> anyhow::Result<Page> {
    let header = parse_page_header(buffer)?;

    let content_buffer = &buffer[PAGE_LEAF_HEADER_SIZE..];
    let cell_pointers = parse_cell_pointers(content_buffer, header.cell_count as usize, ptr_offset);

    let cells = cell_pointers
        .iter()
        .map(|&ptr| parse_table_leaf_cell(&buffer[ptr as usize..]))
        .collect::<anyhow::Result<Vec<TableLeafCell>>>()?;

    Ok(
        Page::TableLeaf(
            TableLeafPage {
                header,
                cell_pointers,
                cells,
            }
        )
    )
}

// 0th byte => 13 for a table btree leaf
// word at 1 byte offset => first free block offset in the page, 0 if no free block
// word at 3 byte offset => cell count in the page
// word at 5 byte offset => offset of the first cell
// 7th byte => number of fragmented free bytes in the page (probably used for vaccuuming?)
// 2 words at 8 byte offset => right most pointer in interior b-tree pages
const PAGE_FIRST_FREEBLOCK_OFFSET: usize = 1;
const PAGE_CELL_COUNT_OFFSET: usize = 3;
const PAGE_CELL_CONTENT_OFFSET: usize = 5;
const PAGE_FRAGMENTED_BYTES_COUNT_OFFSET: usize = 7;
fn parse_page_header(buffer: &[u8]) -> anyhow::Result<PageHeader> {
    if buffer.len() < 7 {
        return Err(anyhow::anyhow!("page header must be at least 7-byte long"))
    }

    let page_type = parse_page_type(buffer)?;

    let first_freeblock = read_be_word_at(buffer, PAGE_FIRST_FREEBLOCK_OFFSET);
    let cell_count = read_be_word_at(buffer, PAGE_CELL_COUNT_OFFSET);
    let cell_content_offset = match read_be_word_at(buffer, PAGE_CELL_CONTENT_OFFSET) {
        0 => PAGE_MAX_SIZE,
        n => n as u32,
    };
    let fragmented_bytes_count = buffer[PAGE_FRAGMENTED_BYTES_COUNT_OFFSET];

    Ok(
        PageHeader {
            page_type,
            first_freeblock,
            cell_count,
            cell_content_offset,
            fragmented_bytes_count,
        }
    )
}

// turns [u8] into [u16]
fn parse_cell_pointers(buffer: &[u8], n: usize, ptr_offset: u16) -> Vec<u16> {
    let mut pointers = Vec::with_capacity(n);
    for i in 0..n {
        let offset = 2 * i;
        if offset + 2 <= buffer.len() {
            pointers.push(read_be_word_at(buffer, offset) - ptr_offset);
        } else {
            break;
        }
    }
    pointers
}

// format is:
// - size of the payload: varint
// - row id: varint
// - payload
fn parse_table_leaf_cell(mut buffer: &[u8]) -> anyhow::Result<TableLeafCell> {
    let (n, size) = read_varint_at(buffer, 0);
    buffer = &buffer[n as usize..];

    let (n, row_id) = read_varint_at(buffer, 0);
    buffer = &buffer[n as usize..];

    let su = size as usize;
    let payload = if su <= buffer.len() {
        buffer[..su].to_vec()
    } else {
        buffer.to_vec()
    };

    Ok(
        TableLeafCell {
            size,
            row_id,
            payload,
        }
    )
}

// https://sqlite.org/src4/doc/trunk/www/varint.wiki
//pub fn read_varint(buffer: &[u8], mut offset: usize) -> (u8, i64) {
//    let mut size = 0;
//    
//    let head = buffer[offset];
//    match head {
//        a0 if a0 >= 0 && a0 <= 240 => (1, a0 as i64),
//        a0 if a0 >= 241 && a0 <= 248 => (2, (240 + 256 * (a0 - 241) + buffer[offset + 1]) as i64),
//        249 => (3, (2288 + 256 * buffer[offset + 1] + buffer[offset + 2]) as i64),
//        250 => (3, sum_buffer(buffer, offset, 3)),
//        251 => (4, sum_buffer(buffer, offset, 4)),
//        252 => (5, sum_buffer(buffer, offset, 5)),
//        253 => (6, sum_buffer(buffer, offset, 6)),
//        254 => (7, sum_buffer(buffer, offset, 7)),
//        255 => (8, sum_buffer(buffer, offset, 8)),
//    }
//}

//fn sum_buffer(buffer: &[u8], offset: usize, bytes: u8) -> i64 {
//    let mut res = 0;
//    for i in 1..bytes {
//        res += buffer[offset + i as usize];
//    }
//    res
//}

pub fn read_varint_at(buffer: &[u8], offset: usize) -> (u8, i64) {
    let mut res: i64 = 0;
    let mut bytes: u8 = 0;

    if offset < buffer.len() {
        for (i, byte) in buffer[offset..].iter().enumerate().take(9) {
            bytes += 1;
            if i == 8 {
                res = (res << 8) | *byte as i64;
                break;
            } else {
                res = (res << 7) | (*byte & 0b0111_1111) as i64;
                if *byte < 0b1000_0000 {
                    break;
                }
            }
        }
    }
    (bytes, res)
}

#[allow(dead_code)]
fn read_varint_rec(buffer: &[u8], offset: usize) -> (u8, i64) {
    fn go(buffer: &[u8], offset: usize, res: i64, bytes: u8) -> (u8, i64) {
        if offset + bytes as usize >= buffer.len() {
            (bytes, res)
        } else {
            let byte = buffer[offset + bytes as usize];
            let b = bytes + 1;

            if b == 9 {
                (b, (res << 8) | byte as i64)
            } else {
                let r = (res << 7) | (byte & 0b0111_1111) as i64;
                if byte < 0b1000_0000 {
                    (b, r)
                } else {
                    go(buffer, offset, r, b)
                }
            }
        }
    }

    go(buffer, offset, 0, 0)
}

// 2: interior index b-tree page
// 5: interior table b-tree page
// 10: leaf index b-tree page
// 13: leaf table b-tree page
const PAGE_LEAF_TABLE_ID: u8 = 13;
fn parse_page_type(buffer: &[u8]) -> anyhow::Result<PageType> {
    match buffer[0] {
        PAGE_LEAF_TABLE_ID => Ok(PageType::TableLeaf),
        _ => Err(anyhow::anyhow!("unknown page type: {}", buffer[0])),
    }
}

fn read_be_word_at(input: &[u8], offset: usize) -> u16 {
    let len = input.len();
    if len >= offset + 2 {
        u16::from_be_bytes(input[offset..offset + 2].try_into().unwrap())
    } else if len > offset {
        input[offset] as u16
    } else {
        0
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_table_leaf_page_tests() -> () {
        assert!(parse_table_leaf_page(&[12], 0).is_err());
        let buffer = [
            // page header w/ 1 as cell count
            13, 0, 12, 0, 1, 0, 0, 0,
            // cell pointer
            0, 10,
            // leaf cell (size, row id, payload)
            10, 2, 127
        ];
        let res = parse_table_leaf_page(&buffer, 0);
        assert!(res.is_ok());
        let expected = Page::TableLeaf(
            TableLeafPage {
                header: PageHeader {
                    page_type: PageType::TableLeaf,
                    first_freeblock: 12,
                    cell_count: 1,
                    cell_content_offset: 65536,
                    fragmented_bytes_count: 0,
                },
                cell_pointers: vec![10],
                cells: vec!{TableLeafCell { size: 10, row_id: 2, payload: vec![127]}},
            }
        );
        assert_eq!(expected, res.unwrap());
    }

    #[test]
    fn parse_page_header_tests() -> () {
        // first byte must be 13 for a table b-tree leaf
        assert!(parse_page_header(&[12]).is_err());
        assert!(parse_page_header(&[12, 0, 12, 0, 11, 0, 10, 0]).is_err());
        let res = parse_page_header(&[13, 0, 12, 0, 11, 0, 0, 0]);
        assert!(res.is_ok());
        let expected = PageHeader {
            page_type: PageType::TableLeaf,
            first_freeblock: 12,
            cell_count: 11,
            cell_content_offset: 65536,
            fragmented_bytes_count: 0,
        };
        assert_eq!(expected, res.unwrap());
    }

    #[test]
    fn parse_cell_pointers_test() -> () {
        assert_eq!(vec![65535], parse_cell_pointers(&[255, 255], 1, 0));
        assert_eq!(vec![65535], parse_cell_pointers(&[255, 255], 2, 0));
        assert_eq!(vec![65435], parse_cell_pointers(&[255, 255], 1, HEADER_SIZE as u16));
    }

    #[test]
    fn parse_table_leaf_cell_tests() -> () {
        let size = 10;
        let row_id = 2;
        let payload = 127;
        let input = [size, row_id, payload];
        let res = parse_table_leaf_cell(&input);
        let expected = TableLeafCell { size: size as i64, row_id: row_id as i64, payload: vec![payload] };
        assert!(res.is_ok());
        assert_eq!(expected, res.unwrap());
    }

    #[test]
    fn read_single_byte_varint() {
        assert_eq!((1, 1), read_varint_at(&vec![0b00000001], 0));
        assert_eq!((1, 3), read_varint_at(&vec![0b00000011], 0));
        assert_eq!((1, 7), read_varint_at(&vec![0b00000111], 0));
        assert_eq!((1, 15), read_varint_at(&vec![0b00001111], 0));
        assert_eq!((1, 127), read_varint_at(&vec![0b11111111], 0));
        assert_eq!((1, 1), read_varint_rec(&vec![0b00000001], 0));
        assert_eq!((1, 3), read_varint_rec(&vec![0b00000011], 0));
        assert_eq!((1, 7), read_varint_rec(&vec![0b00000111], 0));
        assert_eq!((1, 15), read_varint_rec(&vec![0b00001111], 0));
        assert_eq!((1, 127), read_varint_rec(&vec![0b11111111], 0));
    }

    #[test]
    fn read_two_byte_varint() {
        assert_eq!((2, 128), read_varint_at(&vec![0b10000001, 0b00000000], 0));
        assert_eq!((2, 129), read_varint_at(&vec![0b10000001, 0b00000001], 0));
        assert_eq!((2, 255), read_varint_at(&vec![0b10000001, 0b01111111], 0));
        assert_eq!((2, 128), read_varint_rec(&vec![0b10000001, 0b00000000], 0));
        assert_eq!((2, 129), read_varint_rec(&vec![0b10000001, 0b00000001], 0));
        assert_eq!((2, 255), read_varint_rec(&vec![0b10000001, 0b01111111], 0));
    }

    #[test]
    fn read_nine_byte_varint() {
        assert_eq!((9, -1), read_varint_at(&vec![0xff; 9], 0));
        assert_eq!((9, -1), read_varint_rec(&vec![0xff; 9], 0));
    }

    #[test]
    fn read_varint_in_longer_bytes() {
        assert_eq!((1, 1), read_varint_at(&vec![0x01; 10], 0));
        assert_eq!((9, -1), read_varint_at(&vec![0xff; 10], 0));
        assert_eq!((1, 1), read_varint_rec(&vec![0x01; 10], 0));
        assert_eq!((9, -1), read_varint_rec(&vec![0xff; 10], 0));
        println!("{:?}", &vec![0xff; 10]);
    }

    #[test]
    fn read_varint_at_short() -> () {
        assert_eq!((1, 127), read_varint_at(&[255], 0));
        assert_eq!((1, 127), read_varint_rec(&[255], 0));
    }

    #[test]
    fn read_varint_at_empty() -> () {
        assert_eq!((0, 0), read_varint_at(&[], 0));
        assert_eq!((0, 0), read_varint_rec(&[], 0));
    }

    #[test]
    fn read_varint_at_offset() -> () {
        assert_eq!((0, 0), read_varint_at(&[], 1));
        assert_eq!((0, 0), read_varint_rec(&[], 1));
        assert_eq!((1, 127), read_varint_at(&vec![0b10000001, 0b01111111], 1));
        assert_eq!((1, 127), read_varint_rec(&vec![0b10000001, 0b01111111], 1));
    }

    #[test]
    fn parse_page_type_not_leaf() -> () {
        let input = [12];
        let res = parse_page_type(&input);
        assert!(res.is_err());
        assert_eq!(format!("{}", res.unwrap_err()), "unknown page type: 12");
    }

    #[test]
    fn parse_page_type_leaf() -> () {
        let input = [13];
        let res = parse_page_type(&input);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), PageType::TableLeaf);
    }

    #[test]
    fn read_be_word_at_0_offset() -> () {
        let input = [12, 14];
        let res = read_be_word_at(&input, 0);
        assert_eq!(res, 3086);
    }

    #[test]
    fn read_be_word_at_offset() -> () {
        assert_eq!(3086, read_be_word_at(&[255, 12, 14], 1));
        assert_eq!(255, read_be_word_at(&[255], 0));
        assert_eq!(0, read_be_word_at(&[255], 1));
    }
}
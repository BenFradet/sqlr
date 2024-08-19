use crate::page;

pub const HEADER_SIZE: usize = 100;
const HEADER_PREFIX: &[u8] = b"SQLite format 3\0";
const HEADER_PAGE_SIZE_OFFSET: usize = 16;
const PAGE_MAX_SIZE: u32 = 65536;

pub fn parse_header(buffer: &[u8]) -> anyhow::Result<page::DbHeader> {
    if !buffer.starts_with(HEADER_PREFIX) {
        let prefix = String::from_utf8_lossy(&buffer[..HEADER_PREFIX.len()]);
        Err(anyhow::anyhow!("invalid header prefix: {prefix}"))
    } else {
        let page_size_raw = read_word_at(buffer, HEADER_PAGE_SIZE_OFFSET);
        let page_size = match page_size_raw {
            1 => Ok(PAGE_MAX_SIZE),
            n if n.is_power_of_two() => Ok(n as u32),
            _ => Err(anyhow::anyhow!("page size is not a power of 2: {}", page_size_raw)),
        };
        page_size.map(|page_size| page::DbHeader { page_size })
    }
}

fn read_word_at(input: &[u8], offset: usize) -> u16 {
    u16::from_be_bytes(input[offset..offset + 2].try_into().unwrap())
}
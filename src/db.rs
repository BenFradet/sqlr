use std::{io::Read, path::Path};

use anyhow::Context;

use crate::{page::{self, page_header, pager::Pager}, scanner::Scanner, utils};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct DbHeader {
    pub page_size: u32,
}

impl DbHeader {
    const HEADER_PREFIX: &'static [u8] = b"SQLite format 3\0";
    const HEADER_PAGE_SIZE_OFFSET: usize = 16;

    pub fn parse(buffer: &[u8]) -> anyhow::Result<DbHeader> {
        if !buffer.starts_with(Self::HEADER_PREFIX) {
            let len = buffer.len().min(Self::HEADER_PREFIX.len());
            let prefix = String::from_utf8_lossy(&buffer[..len]);
            Err(anyhow::anyhow!("invalid header prefix: {prefix}"))
        } else {
            let page_size_raw = utils::read_be_word_at(buffer, Self::HEADER_PAGE_SIZE_OFFSET).1;
            let page_size = match page_size_raw {
                1 => Ok(page_header::PAGE_MAX_SIZE),
                n if n.is_power_of_two() => Ok(n as u32),
                _ => Err(anyhow::anyhow!(
                    "page size is not a power of 2: {}",
                    page_size_raw
                )),
            };
            page_size.map(|page_size| DbHeader { page_size })
        }
    }
}

#[derive(Debug)]
pub struct Db {
    pub header: DbHeader,
    pager: Pager,
}

impl Db {
    pub fn from_file(filename: impl AsRef<Path>) -> anyhow::Result<Db> {
        let mut file = std::fs::File::open(filename.as_ref()).context("open db file")?;

        let mut header_buffer = [0; page::page::HEADER_SIZE];
        file.read_exact(&mut header_buffer)
            .context("read db header")?;

        let header = DbHeader::parse(&header_buffer).context("parse db header")?;

        let pager = Pager::new(file, header.page_size as usize);

        Ok(Db { header, pager })
    }

    pub fn scanner(&mut self, page: usize) -> Scanner {
        Scanner::new(&mut self.pager, page)
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn parse_dbheader_tests() -> () {
        assert!(DbHeader::parse(&[1, 2, 3]).is_err());
        assert!(DbHeader::parse(&[DbHeader::HEADER_PREFIX, &[3]].concat()).is_err());
        let res_max = DbHeader::parse(&[DbHeader::HEADER_PREFIX, &[1]].concat());
        assert!(res_max.is_ok());
        assert_eq!(DbHeader { page_size: 65536 }, res_max.unwrap());
        let res_pow = DbHeader::parse(&[DbHeader::HEADER_PREFIX, &[8]].concat());
        assert!(res_pow.is_ok());
        assert_eq!(DbHeader { page_size: 8 }, res_pow.unwrap());
    }

    #[test]
    fn from_file_tests() -> () {
        let res = Db::from_file("test.db");
        assert!(res.is_ok());
        let db = res.unwrap();
        assert_eq!(DbHeader { page_size: 4096 }, db.header);
        assert_eq!(4096, db.pager.page_size);
        assert_eq!(HashMap::new(), db.pager.pages);
    }
}

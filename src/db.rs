use std::{io::Read, path::Path};

use anyhow::Context;

use crate::{
    page::{self, DbHeader},
    pager::Pager,
    scanner::Scanner,
};

#[derive(Debug)]
pub struct Db {
    pub header: DbHeader,
    pager: Pager,
}

impl Db {
    pub fn from_file(filename: impl AsRef<Path>) -> anyhow::Result<Db> {
        let mut file = std::fs::File::open(filename.as_ref()).context("open db file")?;

        let mut header_buffer = [0; page::HEADER_SIZE];
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
    fn from_file_tests() -> () {
        let res = Db::from_file("test.db");
        assert!(res.is_ok());
        let db = res.unwrap();
        assert_eq!(DbHeader { page_size: 4096 }, db.header);
        assert_eq!(4096, db.pager.page_size);
        assert_eq!(HashMap::new(), db.pager.pages);
    }
}

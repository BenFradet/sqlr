use std::{
    collections::{hash_map::Entry, HashMap},
    io::{Read, Seek},
};

use anyhow::Context;

use crate::page::page::{self, Page};

#[derive(Debug, Clone)]
pub struct Pager<I: Read + Seek = std::fs::File> {
    input: I,
    pub page_size: usize,
    pub pages: HashMap<usize, Page>,
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
        if let Entry::Vacant(_) = self.pages.entry(n) {
            let page = self.load_page(n)?;
            self.pages.insert(n, page);
        }
        Ok(self.pages.get(&n).unwrap())
    }

    fn load_page(&mut self, n: usize) -> anyhow::Result<Page> {
        let offset = page::HEADER_SIZE + n.saturating_sub(1) * self.page_size;

        self.input
            .seek(std::io::SeekFrom::Start(offset as u64))
            .context("seek to page start")?;

        let mut buffer = vec![0; self.page_size];
        self.input.read_exact(&mut buffer).context("read page")?;

        Page::parse(&buffer, n)
    }
}

#[cfg(test)]
mod test {
    use crate::page::{cell::TableLeafCell, page_header::PageHeader};

    use super::*;

    #[test]
    fn load_page_tests() -> () {
        let file = std::fs::File::open("test.db").unwrap();
        let mut pager = Pager::new(file, 4096);
        assert!(pager.load_page(2).is_err());
        let file = std::fs::File::open("test.db").unwrap();
        let mut pager = Pager::new(file, 8192);
        assert!(pager.load_page(0).is_err());
        let file = std::fs::File::open("test_wrong_page_type.db").unwrap();
        let mut pager = Pager::new(file, 4096);
        assert!(pager.load_page(0).is_err());
        let file = std::fs::File::open("test.db").unwrap();
        let mut pager = Pager::new(file, 4096);
        let page = pager.load_page(1);
        assert!(page.is_ok());
        assert_eq!(
            page.unwrap(),
            Page {
                header: PageHeader::TableLeafPageHeader {
                    first_freeblock: 0,
                    cell_count: 1,
                    cell_content_offset: 4038,
                    fragmented_bytes_count: 0,
                },
                cell_pointers: vec![3938],
                cells: vec![TableLeafCell {
                    size: 56,
                    row_id: 1,
                    payload: vec![
                        6, 23, 21, 21, 1, 85, 116, 97, 98, 108, 101, 116, 98, 108, 49, 116, 98,
                        108, 49, 2, 67, 82, 69, 65, 84, 69, 32, 84, 65, 66, 76, 69, 32, 116, 98,
                        108, 49, 40, 111, 110, 101, 32, 116, 101, 120, 116, 44, 32, 116, 119, 111,
                        32, 105, 110, 116, 41
                    ]
                }.into()]
            },
        )
    }

    #[test]
    fn read_page_tests() -> () {
        let file = std::fs::File::open("test.db").unwrap();
        let mut pager = Pager::new(file, 4096);
        let pages = pager.pages.clone();
        assert_eq!(pages.len(), 0);
        let res = pager.read_page(1);
        assert!(res.is_ok());
        let page = res.unwrap().clone();
        let pages = pager.pages;
        assert_eq!(pages.len(), 1);
        let page_opt = pages.get(&1).cloned();
        assert_eq!(Some(page), page_opt);
    }
}

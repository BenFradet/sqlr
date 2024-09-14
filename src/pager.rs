use std::{collections::HashMap, io::{Read, Seek}};

use anyhow::Context;

use crate::page::*;

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

        Page::parse(&buffer, n)
    }
}
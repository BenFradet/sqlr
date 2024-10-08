use crate::{
    cursor::{Cursor, RecordHeader},
    page::Page,
    pager::Pager,
};

#[derive(Debug)]
pub struct Scanner<'p> {
    pager: &'p mut Pager,
    page: usize,
    cell: usize,
}

impl<'p> Scanner<'p> {
    pub fn new(pager: &'p mut Pager, page: usize) -> Scanner<'p> {
        Scanner {
            pager,
            page,
            cell: 0,
        }
    }

    pub fn next_record(&mut self) -> Option<anyhow::Result<Cursor>> {
        let page = match self.pager.read_page(self.page) {
            Ok(page) => page,
            Err(e) => return Some(Err(e)),
        };

        match page {
            Page::TableLeaf(leaf) => {
                let cell = leaf.cells.get(self.cell)?;

                let header = match RecordHeader::parse(&cell.payload) {
                    Ok(header) => header,
                    Err(e) => return Some(Err(e)),
                };

                let record = Cursor::new(header, self.pager, self.page, self.cell);

                self.cell += 1;

                Some(Ok(record))
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn next_record_tests() -> () {
        let file = std::fs::File::open("test.db").unwrap();
        let mut pager = Pager::new(file, 4096);
        let mut scanner = Scanner::new(&mut pager, 4);
        let res = scanner.next_record();
        assert!(res.is_some());
        assert!(res.unwrap().is_err());
        let mut scanner = Scanner::new(&mut pager, 1);
        let res = scanner.next_record();
        assert!(res.is_some());
        assert!(res.unwrap().is_ok());
        let res = scanner.next_record();
        assert!(res.is_none());
    }
}

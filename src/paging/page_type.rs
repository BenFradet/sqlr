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
    pub fn parse(buffer: &[u8]) -> anyhow::Result<PageType> {
        match buffer[0] {
            Self::PAGE_LEAF_TABLE_ID => Ok(PageType::TableLeaf),
            Self::PAGE_INTERIOR_TABLE_ID => Ok(PageType::TableInterior),
            _ => Err(anyhow::anyhow!("unknown page type: {}", buffer[0])),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_page_type_tests() -> () {
        assert!(PageType::parse(&[12]).is_err());
        let res = PageType::parse(&[13]);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), PageType::TableLeaf);
    }
}

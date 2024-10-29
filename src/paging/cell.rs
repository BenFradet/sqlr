use crate::utils;

#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    TableLeaf(TableLeafCell),
    TableInterior(TableInteriorCell),
}

impl From<TableLeafCell> for Cell {
    fn from(cell: TableLeafCell) -> Self {
        Cell::TableLeaf(cell)
    }
}

impl From<TableInteriorCell> for Cell {
    fn from(cell: TableInteriorCell) -> Self {
        Cell::TableInterior(cell)
    }
}

// cells in an interior page are ordered by key
#[derive(Debug, Clone, PartialEq)]
pub struct TableInteriorCell {
    // points to a child page that contains keys <= key
    pub left_child_page: u32,
    // integer key or row id
    pub key: i64,
}

impl TableInteriorCell {
    pub fn parse(mut buffer: &[u8]) -> anyhow::Result<Cell> {
        let (n, left_child_page) = utils::read_be_double_word_at(buffer, 0);
        buffer = &buffer[n as usize..];

        let (_, key) = utils::read_varint_at(buffer, 0);
        Ok(TableInteriorCell {
            left_child_page,
            key,
        }
        .into())
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
    pub fn parse(mut buffer: &[u8]) -> anyhow::Result<Cell> {
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
        }
        .into())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_table_leaf_cell_tests() -> () {
        let size = 10;
        let row_id = 2;
        let payload = 127;
        let input = [size, row_id, payload];
        let res = TableLeafCell::parse(&input);
        let expected = Cell::TableLeaf(TableLeafCell {
            size: size as i64,
            row_id: row_id as i64,
            payload: vec![payload],
        });
        assert!(res.is_ok());
        assert_eq!(expected, res.unwrap());
    }

    #[test]
    fn parse_table_interior_cell_tests() -> () {
        let left_child_page = 10;
        let key = 127;
        let input = [0, 0, 0, left_child_page, key];
        let res = TableInteriorCell::parse(&input);
        let expected = Cell::TableInterior(TableInteriorCell {
            left_child_page: left_child_page as u32,
            key: key as i64,
        });
        assert!(res.is_ok());
        assert_eq!(expected, res.unwrap());
    }
}

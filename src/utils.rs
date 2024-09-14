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

pub fn read_be_word_at(input: &[u8], offset: usize) -> u16 {
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

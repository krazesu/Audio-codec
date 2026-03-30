pub struct Utf8Encoder;

impl Utf8Encoder {
    /// Encode a number into its UTF-9 equivalent encoding
    /// 
    /// Although UTF-8 encoding is for characters, characters are
    /// mapped to certain numbers.
    pub fn encode(mut num: u64) -> Vec <u8> {
        //get min number of bits to represent the number
        let bitlen = num.checked_ilog2().unwrap_or(0) + 1;

        let mut UTF8_encoded: Vec<u8> = vec![];
        let mut ctr = 0;

        // encode in UTF-8 using the Template based on number of bits to represent the number to be encoded
        match bitlen{
            0..=7 => {
                let temp: u8 =  0b0000_0000 | num as u8;
                UTF8_encoded.insert(0, temp);

                return UTF8_encoded;
            },
            8..=11 => ctr = 1, 
            12..=16 => ctr = 2,
            17..=21 => ctr = 3,
            22..=26 => ctr = 4,
            27..=31 => ctr = 5,
            32..=40 => ctr = 6,
            _ => todo!(),
        }

        //insert bits from the number to be encoded into the template   
        //insert every 6 bits from the number into byte-template: 10xx xxxx
        for x in 1..=ctr{ 
            let ins: u64 = 0b0011_1111 & num;
            num = num >> 6;
            let temp: u8 =  0b1000_0000 | ins as u8;
            UTF8_encoded.insert(0, temp);
        }

        //insert remaining bits to the last byte-template depending on # of bits
        let ins: u64 = (0b1111_1111) & num;
        let temp: u8 =  (0b1111_1110 << (6-ctr)) | ins as u8;
        UTF8_encoded.insert(0, temp);
        return UTF8_encoded;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_01() {
        let in_val = 0;
        let out_val_ans = vec![0u8];
        let out_val = Utf8Encoder::encode(in_val);

        assert_eq!(out_val_ans, out_val);
    }

    #[test]
    fn sample_02() {
        let in_val = 0x164;
        let out_val_ans = vec![0xc5u8, 0xa4u8];
        let out_val = Utf8Encoder::encode(in_val);

        assert_eq!(out_val_ans, out_val);
    }

    #[test]
    fn sample_03() {
        let in_val = 0x7F; // 7 bits
        let out_val_ans = vec![0x7Fu8];
        let out_val = Utf8Encoder::encode(in_val);
        assert_eq!(out_val_ans, out_val);
    }

    #[test]
    fn sample_04() {
        let in_val = 0x80; // 8 bits
        let out_val_ans = vec![0xc2u8, 0x80u8];
        let out_val = Utf8Encoder::encode(in_val);
        assert_eq!(out_val_ans, out_val);
    }

    #[test]
    fn sample_05() {
        
    let in_val = 0x7FF; // 11 bits
    let out_val_ans = vec![0xdfu8, 0xbfu8];
    let out_val = Utf8Encoder::encode(in_val);
    assert_eq!(out_val_ans, out_val);
    }

    #[test]
    fn sample_06() {
        let in_val = 0x800; // 12 bits
        let out_val_ans = vec![0xe0u8, 0xa0u8, 0x80u8];
        let out_val = Utf8Encoder::encode(in_val);
        assert_eq!(out_val_ans, out_val);
    }

    #[test]
    fn sample_07() {
        let in_val = 0xFFFF; // 16 bits
        let out_val_ans = vec![0xefu8, 0xbfu8, 0xbfu8];
        let out_val = Utf8Encoder::encode(in_val);
        assert_eq!(out_val_ans, out_val);
    }

    #[test]
    fn sample_08() {
        let in_val = 0x10000; // 17 bits
        let out_val_ans = vec![0xf0u8, 0x90u8, 0x80u8, 0x80u8];
        let out_val = Utf8Encoder::encode(in_val);
        assert_eq!(out_val_ans, out_val);
    }

    #[test]
    fn sample_09() {
        let in_val = 0x79beff; // 23 bits
        let out_val_ans = vec![0xf8u8, 0x9eu8, 0x9bu8, 0xbbu8, 0xbfu8];
        let out_val = Utf8Encoder::encode(in_val);
        assert_eq!(out_val_ans, out_val);
    }

    #[test]
    fn sample_10() {
        let in_val = 0x80000000; // 32 bits
        let out_val_ans = vec![0xfeu8, 0x82u8, 0x80u8, 0x80u8, 0x80u8, 0x80u8, 0x80u8];
        let out_val = Utf8Encoder::encode(in_val);
        assert_eq!(out_val_ans, out_val);
    }
}

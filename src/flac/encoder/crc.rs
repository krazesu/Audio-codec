/// Represents a kind of CRC encoding
/// 
/// This struct is used to configure the type of CRC encoding to use.
/// For example, if the generator polynomial for a CRC8 encoding is:
/// 
/// `x^8 + x^2 + x^1 + 1`
/// 
/// Then, the value of `poly` should be 0b0000_0111 (note the missing
/// MSB `1` bit) and `poly_len` should be `u8`.
pub struct CrcOptions <T> {
    poly: T,
    poly_len: T,
}


impl <T> CrcOptions <T> {
    /// Create a builder to the CRC encoder
    pub fn new(poly: T, poly_len: T) -> Self {
        return Self{
            poly,
            poly_len,
        }
    }
}

impl CrcOptions <u8> {
    /// Encode data using CRC8 encoding
    /// 
    /// This method is available only if `CrcOptions` is of type `u8`.
    pub fn build_crc8(&self, data: &Vec <u8>) -> u8 {
        let mut crc:u8 = 0;
        let mut temp:Vec<u8> = Vec::new();
        let mut data_vec: Vec<u8> = Vec::new();

        // create a vector of bits representing the data
        for byte in data{
            for idx in 0..self.poly_len{
                let bit = (byte & (0b1000_0000 >> idx)) >> (7-idx);
                data_vec.insert(0, bit);
            }
        }

        // Attach L number of zeros to the right of data
        for _ in 0..8{
            data_vec.insert(0, 0);
        }

        //perform XOR bit for bit for every P= 1_0000_0111 (9 bits)
        while data_vec.len() > 0 || temp.len() == 9{
            if temp.len() == 9{
                let poly = self.poly;
                let mut check = 0;
                
                for i in 0..9{
                    let temp_bit = temp.pop().unwrap_or(0);
                    let mut poly_bit:u8 = 0;

                    if i == 0{
                        poly_bit = 1;
                    }
                    else if poly & (0b1000_0000 >>(i-1)) != 0{
                        poly_bit = 1;
                    }
                    
                    if temp_bit ^ poly_bit == 1{
                        check = 1;
                    }  
                    if check == 1{
                        temp.insert(0, temp_bit ^ poly_bit);
                    }
                }
            }
            else{
                temp.insert(0, data_vec.pop().unwrap_or(0));
            }
        }
        
        //convert vec of bits to CRC8 value (u8) 
        for _ in 0..temp.len(){
            crc = (crc << 1) | temp.pop().unwrap_or(0);
        }

        crc
    }
}

impl CrcOptions <u16> {
    /// Encode data using CRC16 encoding
    /// 
    /// This method is available only if `CrcOptions` is of type `u16`.
    pub fn build_crc16(&self, data: &Vec <u8>) -> u16 {
        let mut crc:u16 = 0;
        let mut temp:Vec<u8> = Vec::new();
        let mut data_vec: Vec<u8> = Vec::new();

        // create a vector of bits representing the data
        for word in data{
            for idx in 0..8{ //// change self.poly -> 8 
                let bit = (word & (0b1000_0000 >> idx)) >> (7-idx); //// change to 0b1000_0000 and 15->7

                data_vec.insert(0, bit);
            }
        }
        
        // Attach L number of zeros to the right of data
        for _ in 0..16{
            data_vec.insert(0, 0);
        }

        //perform XOR bit for bit for every P=1_1000_0000_0000_0101 (17 bits)
        while data_vec.len() > 0 || temp.len() == 17{
            if temp.len() == 17{
                let mut poly = self.poly;
                let mut check = 0;
                for i in 0..17{
                    let temp_bit = temp.pop().unwrap_or(0);
                    let mut poly_bit:u8 = 0;

                    if i == 0{
                        poly_bit = 1;
                    }
                    else if poly & (0b1000_0000_0000_0000 >> (i-1)) != 0{
                        poly_bit = 1
                    }

                    if temp_bit ^ poly_bit == 1{
                        check = 1;
                    }  
                    if check == 1{
                        temp.insert(0, temp_bit ^ poly_bit);
                    }
                }
            }
            else{
                temp.insert(0, data_vec.pop().unwrap_or(0));
            }
        }
        
        //convert vec of bits to CRC8 value (u8) 
        for _ in 0..temp.len(){
            crc = (crc << 1) | temp.pop().unwrap_or(0) as u16;
        }

        crc   
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_crc8_01() {
        let in_vec = vec![
            0x10,
        ];
        let ans = CrcOptions::new(0b0000_0111u8, 8)
            .build_crc8(&in_vec);

        assert_eq!(ans, 0x70);
    }

    #[test]
    fn sample_crc8_ietf_01() {
        let in_vec = vec![
            0xff, 0xf8, 0x69, 0x18,
            0x00, 0x00,
        ];
        let ans = CrcOptions::new(0b0000_0111u8, 8)
            .build_crc8(&in_vec);

        assert_eq!(ans, 0xbf);
    }

    #[test]
    fn sample_crc16_01() {
        let in_vec = vec![
            0x10, 0x00,
        ];
        let ans = CrcOptions::new(0b1000_0000_0000_0101u16, 16)
            .build_crc16(&in_vec);

        assert_eq!(ans, 0xe003);
    }

    #[test]
    fn sample_crc16_ietf_01() {
        let in_vec = vec![
            0xff, 0xf8, 0x69, 0x18,
            0x00, 0x00, 0xbf, 0x03,
            0x58, 0xfd, 0x03, 0x12,
            0x8b,
        ];
        let ans = CrcOptions::new(0b1000_0000_0000_0101u16, 16)
            .build_crc16(&in_vec);

        assert_eq!(ans, 0xaa9a);
    }

    //Additional Testcases
    #[test]
    fn sample_crc8_02() {
        let in_vec = vec![
            0x10, 0x20, 0x30, 0x40,
            0x50, 0x60, 0x70, 0x80,
            0x90, 0x00,
        ];
        let ans = CrcOptions::new(0b0000_0111u8, 8)
            .build_crc8(&in_vec);

        assert_eq!(ans, 0x1F);
    }

    #[test]
    fn sample_crc8_03() {
        let in_vec = vec![
            0xfa, 0xfa, 0xc0,
        ];
        let ans = CrcOptions::new(0b0000_0111u8, 8)
            .build_crc8(&in_vec);

        assert_eq!(ans, 0x33);
    }

    #[test]
    fn sample_crc16_02() {
        let in_vec = vec![
            0xff, 0x34, 0xf0, 0x12,
            0x31,
        ];
        let ans = CrcOptions::new(0b1000_0000_0000_0101u16, 16)
            .build_crc16(&in_vec);

        assert_eq!(ans, 0xD844);
    }

    fn sample_crc16_03() {
        let in_vec = vec![
            0x10, 0x20, 0x30, 0x40,
            0x50, 0x60, 0x70, 0x80,
            0x90, 0x00,
        ];
        let ans = CrcOptions::new(0b1000_0000_0000_0101u16, 16)
            .build_crc16(&in_vec);

        assert_eq!(ans, 0xB2B6);
    }
}
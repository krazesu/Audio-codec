use std::ops::{Shl, Shr};
//use crate::flac::bitstream;

/// Represents a Rice encoder
///
/// This encoder is expected to encode `num_samples` residuals from a predictor of
/// order `predictor_order`. Note that Rice encoding in FLAC is only available
/// for LPC and FIXED audio subframes.
pub struct RiceEncoderOptions {
    num_samples: u64,
    predictor_order: u8,
}

/// Represents a Rice-encoded stream
///
/// Rice encoding is _not necessarily_ byte-aligned. The `extra_bits_len`
/// value denotes the number of LSBits in the last byte of the `stream`
/// that are _not_ part of the encoding.
#[derive(Debug)]
pub struct RiceEncodedStream {
    pub stream: Vec <u8>,
    pub param: u8,
    pub extra_bits_len: u8,
}

impl RiceEncoderOptions {
    /// Create a builder to the Rice encoder
    pub fn new(num_samples: u64, predictor_order: u8) -> Self {
        Self {
            num_samples,
            predictor_order,
        }
    }

    /// Get the minimum partition order
    /// 
    /// The default minimum partition order is zero
    fn min_rice_partition_order() -> u8 {
        0
    }

    /// Get the maximum partition order
    /// 
    /// The maximum partition order is computed as the lowest power of two
    /// that makes up the block size, or the index of the least significant
    /// 1 bit in the block size. Note that odd-sized block sizes can only
    /// have a partition order of 0 as the number of partitions should be
    /// a power of two.
    fn max_rice_partition_order(mut block_size: u64) -> u8 {
        let mut position:u8 = 0;

        //search for the position of the first bit 1 from LSB
        while block_size & 1 == 0{
            position +=1 ;
            block_size = block_size >> 1;
        }

        position
    }

    /// Compute the best partition order and best Rice parameters for each partition
    /// 
    /// The best partition order is computed based on the order that yields the minimum
    /// total number of bits of the resulting Rice encoding.
    fn best_partition_and_params(&self, residuals: &Vec <i64>) -> (Vec <u8>, u8) {
        let mut min_num_bits: u64 = u64::MAX;
        let mut best_partition_order:u8 = 0;
        let mut best_rice_parameters:Vec<u8> = Vec::new();

        if self.num_samples%2 == 1{
            let mut best_parameters:(Vec <u8>, u64) = Self::best_parameters(self, 0, &residuals).unwrap_or((vec![], 0));

            return (best_parameters.0, 0 as u8);
        }
        
        for order in 1..=Self::max_rice_partition_order(self.num_samples){
            let mut best_parameters:(Vec <u8>, u64) = Self::best_parameters(self, order, &residuals).unwrap_or((vec![], 0));

            let n_partitions:u64 = self.num_samples / 2u8.pow(order as u32) as u64;
            let n_partition_samples: u64 = 2u8.pow(order as u32) as u64;
            
            let mut num_bits = 0;
            for partition in 0..n_partitions{
                let mut partition_samples:Vec<i64> = Vec::new();
                for s in 0..n_partition_samples{
                    partition_samples.push(residuals[(partition*n_partition_samples) as usize + s as usize]);
                }
                num_bits += Self::bits_in_partition_exact(best_parameters.0[partition as usize], n_partition_samples, &partition_samples);
            }

            if num_bits < min_num_bits{
                best_partition_order = order
            }

            best_rice_parameters = best_parameters.0;
        }
        
        (best_rice_parameters, best_partition_order)
    }

    /// Compute the best Rice parameters for some partition of the residuals
    /// 
    /// The best Rice parameter `M` can be approximated using the following:
    /// 
    /// `M = log2(abs_r_mean - 1) - log2(n_partition_samples) + 1`.
    /// 
    /// Note that in practice, the sum of the absolute value of the residuals
    /// is used instead of the absolute residual mean `abs_r_mean`. In addition,
    /// Most implementations will bound `M` to be represented by at most 18 bits.
    /// 
    /// Note that only partition order 0 is allowed for odd-length residuals
    /// as the number of partitions should be a power of two.
    /// 
    /// # Errors
    /// Returns `None` if a best parameter cannot be found for any partition. This
    /// arises usually if the predictor order is larger than the amount of residuals
    /// in a partition.
    fn best_parameters(&self, partition_order: u8, residuals: &Vec <i64>) -> Option <(Vec <u8>, u64)> {
        let mut best_parameters:Vec<u8> = Vec::new();
        let n_partitions:u64 = self.num_samples / 2u8.pow(partition_order as u32) as u64;
        let n_partition_samples: u64 = 2u8.pow(partition_order as u32) as u64;

        if self.predictor_order > n_partition_samples as u8{
            return None;
        }

        for partition in 0..n_partitions{
            let mut partition_samples:Vec<i64> = Vec::new();
            for s in 0..n_partition_samples{
                partition_samples.push(residuals[(partition*n_partition_samples) as usize + s as usize]);
            }
            let mut abs_residual_sum: u64 = 0;
            let mut best_param:u8 = 0;

            partition_samples.iter().for_each(|x| abs_residual_sum += x.abs() as u64);

            best_param = ((abs_residual_sum).ilog2() + 1 - (n_partition_samples).ilog2()) as u8;

            best_parameters.push(best_param);
        }

        Some((best_parameters, n_partition_samples))
    }

    /// Find the exact total number of bits needed to represent a Rice-encoded
    /// partition of samples
    /// 
    /// A residual `r` can be represented using 1 bit for the unary stop mark,
    /// `rice_param` bits for the truncated binary part of the rice encoding, and
    /// `zigzag(r) >> rice_param` bits for the unary tally marks.
    fn bits_in_partition_exact(rice_param: u8, n_partition_samples: u64, residuals: &Vec<i64>) -> u64 {
        let mut total_number_bits:u64 = 0;
        
        for r in residuals{
            total_number_bits += 1 + rice_param as u64 + (Self::zigzag(*r) >> rice_param) as u64;
        }

        total_number_bits
    }

    /// Find the total number of bits occupied by this encoding
    /// 
    /// Rice encoding uses `q + 1` bits for the unary-encoded quotient `q` and
    /// `rice_param` bits for the binary remainder
    fn bits_in_partition_sums(rice_param: u8, n_partition_samples: u64, abs_residual_sum: u64) -> u64 {
        todo!() 
    }

    /// Encode residuals into Rice encoding
    /// 
    /// To encode a residual into its Rice encoding, it should be first processed
    /// using zigzag encoding so that all of the residuals become nonnegative numbers.
    /// Then, the Rice encoding of each residual is computed.
    /// 
    /// Note that the contents are _not_ ensured to be byte-aligned. Hence, this method returns
    /// the Rice-encoded byte vector containing the number of extra unused bits at the last element.
    pub fn encode(rice_param: u8, residuals: &Vec <i64>) -> RiceEncodedStream {
        let mut stream:Vec<u8> = Vec::new();
        let mut rice_residuals:Vec<u8> = Vec::new();
        let mut extra_bits_len:u8 = 0;

        //for every residual get rice_encoding
        for r in residuals{ 
            let mut r_rep:u64 = 0;

            r_rep = Self::zigzag(*r);
            
            //rice-encode the residual
            rice_residuals.append(&mut Self::rice_encode(rice_param, r_rep));
        } 
        
        //re-group bits of rice-encoded residuals into bytes and store in stream
        let mut bit_ctr:u8 = 1;
        let mut byte:u8 = 0;
        for encoded_r in rice_residuals{
            if bit_ctr == 8{
                byte = (byte<<1) | encoded_r;
                stream.push(byte);
                byte=0;
                bit_ctr=1;
            }
            else{
                byte = (byte<<1) | encoded_r;
                bit_ctr +=1;
            }
        }

        //check if there are unused bits and add the last incomplete byte to stream
        if bit_ctr > 1{
            extra_bits_len = 8-bit_ctr+1;
            byte = byte << extra_bits_len;
            stream.push(byte);
        }

        RiceEncodedStream{
            stream,
            param: rice_param,
            extra_bits_len,
        }
    }

    fn rice_encode(param: u8, sample: u64) -> Vec<u8>{
        let mut vec_rice: Vec<u8> = Vec::new();
        
        //let num_bits_binary = (param.ilog2()) + 1 ;
        let num_bits_binary = param;

        //split the bits of residual and get the first part for unary encoding
        let unary = sample >> num_bits_binary;

        //convert first part to unary(U) using 0 as tally mark
        for _ in 0..unary{
            vec_rice.push(0);
        }
        vec_rice.push(1);

        //get 2nd part, binary(B)
        let mut binary = sample & (2_u64.pow(param as u32) - 1) as u64;

        //pad the binary with zeros to the left to meet bit-length for binary
        let idx = vec_rice.len();
        for _ in 0..num_bits_binary{
            vec_rice.insert(idx, (binary & 1 ) as u8);
            binary = binary >> 1;
        }

        vec_rice
    }

    /// Encode residuals into a partitioned Rice-encoded stream
    /// 
    /// This method computes the Rice encoding of a stream of residuals by first partitioning
    /// the residual into groups. Each group is then found its best Rice parameter and
    /// each residual in the group is then encoded using the parameter.
    /// 
    /// The method returns each Rice-encoded group in chronological order and the partition order,
    /// respectively. The number of elemenets in the vector of Rice-encoded groups should be less than
    /// or equal to `2^partition order`.
    /// 
    /// Note that each of the contents are _not_ ensured to be byte-aligned. Hence, this method
    /// returns the Rice-encoded byte stream and the number of extra unused bits at the last byte
    /// of the stream, respectively.
    pub fn encode_by_partition(&self, residuals: &Vec <i64>)  -> (Vec <RiceEncodedStream>, u8) {
        let parameters_and_order:(Vec <u8>, u8) = Self::best_partition_and_params(self, &residuals);

        let parameters: Vec<u8> = parameters_and_order.0;
        let order: u8 = parameters_and_order.1;

        let n_partitions:u64 = self.num_samples / 2u8.pow(order as u32) as u64;
        let n_partition_samples: u64 = 2u8.pow(order as u32) as u64;

        let mut rice_encoded_residuals:Vec <RiceEncodedStream> = Vec::new();
        for partition in 0..n_partitions{
            let mut partition_samples:Vec<i64> = Vec::new();
            for s in 0..n_partition_samples{
                partition_samples.push(residuals[(partition*n_partition_samples) as usize + s as usize]);
            }
            rice_encoded_residuals.push(Self::encode(parameters[partition as usize], &partition_samples));
        }

        (rice_encoded_residuals, order)
    }

    /// Convert an integer into its zigzag encoding. With this encoding, all
    /// positive numbers are even and all negative numbers are odd.
    pub fn zigzag(num: i64) -> u64 {
         //else if negative,, mupliply by -2 then subtract 1
        if num < 0{
            return (-2*num -1) as u64;
        }
       
        //if residual is positive,, double it
        else{
            return (2*num) as u64;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_sample_ietf_02() {
        let in_vec = vec![
            3194, -1297, 1228, -943,
            952, -696, 768, -524,
            599, -401, -13172, -316,
            274, -267, 134,
        ];

        let out_vec_ans = vec![
            0x11, 0xe8, 0xa2, 0x14,
            0xcc, 0x7a, 0xef, 0xb8,
            0x6b, 0x7f, 0x00, 0x60,
            0xbe, 0x57, 0x59, 0x08,
            0x00, 0x77, 0x3d, 0x3b,
            0xd1, 0x25, 0x0a, 0xc8,
            0x60,
        ];

        let rice_enc_stream = RiceEncoderOptions::encode(11, &in_vec);

        assert_eq!(rice_enc_stream.stream, out_vec_ans);
        assert_eq!(rice_enc_stream.extra_bits_len, 3);
    }

    #[test]
    fn encode_sample_ietf_03() {
        let in_vec = vec![
            3, -1, -13,
        ];

        let out_vec_ans = vec![
            0xe9, 0x12,
        ];

        let rice_enc_stream = RiceEncoderOptions::encode(3, &in_vec);

        assert_eq!(rice_enc_stream.stream, out_vec_ans);
        assert_eq!(rice_enc_stream.extra_bits_len, 1);
    }
}

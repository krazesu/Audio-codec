pub struct FixedPredictor;

impl FixedPredictor {
    /// Get order that yields the least sum of residuals
    /// 
    /// The predictor orders are from 0 to 4 inclusive and is retrieved
    /// by finding the predictor that yields the *minimum* sum of residuals
    /// for the given `data` and derived predictor.
    pub fn best_predictor_order(data: &Vec <i64>) -> Option <u8> {
        let mut min_sum_residual:i64 = i64::MAX;
        let mut min_idx: u8 = 0;
        let mut temp: Vec<i64> = Vec::new();
        
        for i in 0..=4{
            let mut temp_data = data.clone();
            temp = Self::get_residuals(&temp_data, i).unwrap_or(vec![i64::MAX]);
            let sum = temp.iter().sum();
            if sum < min_sum_residual{
                min_sum_residual = sum;
                min_idx = i;
            }
        }
        
        Some(min_idx)
    }

    /// Get residuals of a fixed predictor order 
    /// 
    /// The predictor orders are from 0 to 4 inclusive and corresponds
    /// to one of the five "fixed" predictor orders written in the FLAC
    /// specification. The predictor orders are defined as follows:
    /// 
    /// 0: r[i] = 0
    /// 1: r[i] = data[i - 1]
    /// 2: r[i] = 2 * data[i - 1] - data[i - 2]
    /// 3: r[i] = 3 * data[i - 1] - 3 * data[i - 2] + data[i - 3]
    /// 4: r[i] = 4 * data[i - 1] - 6 * data[i - 2] + 4 data[i - 3] - data[i - 4]
    /// 
    /// This function returns a vector with each element containing data[i] - r[i].
    /// 
    /// # Errors
    /// `None` is returned if an error occurs in the function. This includes whether
    /// the predictor order provided is not within 0 and 4 inclusive and whether the
    /// size of `data` is less than the predictor order.
    pub fn get_residuals(data: &Vec <i64>, predictor_order: u8) -> Option <Vec <i64>> {
        
        if data.len() <= predictor_order as usize || predictor_order > 4 || predictor_order < 0 {
            return None
        }

        let mut residuals:Vec<i64> = Vec::new();
        let mut r: i64 = 0;

        for i in 0..data.len(){
            if i >= predictor_order as usize{
                match predictor_order{
                    0 => r = 0,
                    1 => r = data[i - 1],
                    2 => r = 2 * data[i - 1] - data[i - 2],
                    3 => r = 3 * data[i - 1] - 3 * data[i - 2] + data[i - 3],
                    4 => r = 4 * data[i - 1] - 6 * data[i - 2] + 4 * data[i - 3] - data[i - 4],
                    _ => return None,
                };    
            residuals.push(data[i] - r);
            }
        }

        Some(residuals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_ietf_02a() {
        let in_vec = vec![
            4302, 7496, 6199, 7427,
            6484, 7436, 6740, 7508,
            6984, 7583, 7182, -5990,
            -6306, -6032, -6299, -6165,
        ];

        let out_vec_ans = vec![
            3194, -1297, 1228,
            -943, 952, -696, 768,
            -524, 599, -401, -13172,
            -316, 274, -267, 134,
        ];

        let ans = FixedPredictor::get_residuals(&in_vec, 1);

        assert!(ans.is_some());
        assert_eq!(ans.unwrap(), out_vec_ans);
    }
}
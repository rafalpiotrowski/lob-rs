//!
//!

/// Combine the integer and fractional parts into a float
pub(crate) fn combine_integer_and_fractional(
    integer_part: u64,
    fractional_part: u64,
    precision: u32,
) -> f64 {
    let fractional_multiplier = 10u64.pow(precision);
    integer_part as f64 + (fractional_part as f64 / fractional_multiplier as f64)
}

#[allow(dead_code)]
pub(crate) fn extract_integer_and_fractional(value: f64, precision: u32) -> (u64, u64) {
    let integer_part = value as u64;
    // Extract the fractional part
    let fractional_multiplier = 10i32.pow(precision);
    let fractional_part = ((value - integer_part as f64) * fractional_multiplier as f64) as u64;
    (integer_part, fractional_part)
}

#[allow(dead_code)]
pub(crate) fn u64_to_vec_u8(num: u64, precision: usize) -> Vec<u8> {
    let mut digits: Vec<u8> = num
        .to_string()
        .chars()
        .map(|c| c.to_digit(10).unwrap() as u8)
        .collect();

    // Add leading zeros if the length of the digits is less than the precision
    while digits.len() < precision {
        digits.insert(0, 0);
    }

    digits
}

/// Extract the integer and fractional parts of a float
pub fn f64_to_u128(value: f64) -> u128 {
    // Transmute the f64 to u64 to access the raw bits
    let bits: u64 = value.to_bits();

    // Extract sign (1 bit), exponent (11 bits), and mantissa (52 bits)
    let sign = (bits >> 63) & 1;
    let exponent = (bits >> 52) & 0x7FF;
    let mantissa = bits & 0xFFFFFFFFFFFFF;

    // Reconstruct the u128 value: sign | exponent | mantissa
    let sign_u128 = (sign as u128) << 127;
    let exponent_u128 = (exponent as u128) << 116;
    let mantissa_u128 = mantissa as u128;

    // Combine the components into a u128 value
    sign_u128 | exponent_u128 | mantissa_u128
}

/// Convert a u128 value to an f64 value
pub fn u128_to_f64(value: u128) -> f64 {
    // Extract the sign (1 bit), exponent (11 bits), and mantissa (52 bits)
    let sign = (value >> 127) & 1;
    let exponent = (value >> 116) & 0x7FF;
    let mantissa = value & 0xFFFFFFFFFFFFF;

    // Reconstruct the u64 value from the components
    let sign_u64 = (sign as u64) << 63;
    let exponent_u64 = (exponent as u64) << 52;
    let mantissa_u64 = mantissa as u64;

    // Combine the components into a u64 value
    let bits = sign_u64 | exponent_u64 | mantissa_u64;

    // Transmute the u64 to f64 to get the original floating-point number
    f64::from_bits(bits)
}

mod tests {

    #[test]
    fn test_f64_to_u128_and_back() {
        let value = 21.0453;
        let result = crate::utils::f64_to_u128(value);
        assert_eq!(result, 85319821979444287592014535807444615745);
        let result = crate::utils::u128_to_f64(result);
        assert_eq!(result, value);
    }

    #[test]
    fn test_extracting_integer_and_fractional_parts() {
        let limit = 21.0453;
        let (integer_part, fractional_part) =
            crate::utils::extract_integer_and_fractional(limit, 4);
        assert_eq!(integer_part, 21);
        assert_eq!(fractional_part, 453);

        let result = crate::utils::combine_integer_and_fractional(integer_part, fractional_part, 4);
        assert_eq!(result, limit)
    }

    #[test]
    fn test_u64_to_vec_u8() {
        let num = 344;
        let result = vec![0, 3, 4, 4];
        assert_eq!(result, crate::utils::u64_to_vec_u8(num, 4));
    }
}

//! Reed–Solomon error correction over GF(2^8) with the QR field polynomial
//! x^8 + x^4 + x^3 + x^2 + 1 (0x11D) and generator root 2.

/// Product of two field elements.
pub fn multiply(x: u8, y: u8) -> u8 {
    let mut z: u32 = 0;
    for i in (0..8).rev() {
        z = (z << 1) ^ ((z >> 7) * 0x11D);
        z ^= ((y as u32 >> i) & 1) * x as u32;
    }
    z as u8
}

/// The generator polynomial of `degree`, highest coefficient first and the
/// leading 1 left out: (x - 2^0)(x - 2^1)…(x - 2^(degree-1)).
pub fn divisor(degree: usize) -> Vec<u8> {
    let mut result = vec![0u8; degree];
    result[degree - 1] = 1;
    let mut root: u8 = 1;
    for _ in 0..degree {
        for j in 0..degree {
            result[j] = multiply(result[j], root);
            if j + 1 < degree {
                result[j] ^= result[j + 1];
            }
        }
        root = multiply(root, 0x02);
    }
    result
}

/// The error-correction codewords for `data`: the remainder of data·x^n
/// divided by the generator.
pub fn remainder(data: &[u8], divisor: &[u8]) -> Vec<u8> {
    let mut result = vec![0u8; divisor.len()];
    for &byte in data {
        let factor = byte ^ result.remove(0);
        result.push(0);
        for (slot, &coef) in result.iter_mut().zip(divisor) {
            *slot ^= multiply(coef, factor);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_field_multiplies_as_the_standard_defines() {
        assert_eq!(multiply(0, 0x53), 0);
        assert_eq!(multiply(1, 0x53), 0x53);
        assert_eq!(multiply(2, 0x80), 0x1D); // x^8 folds back into 0x1D
        assert_eq!(multiply(0x53, 0xCA), multiply(0xCA, 0x53));
    }

    /// The widely published worked example: "HELLO WORLD" at 1-M, alphanumeric,
    /// gives these 16 data codewords and these 10 error-correction codewords.
    #[test]
    fn a_published_example_gets_its_error_correction() {
        let data = [32, 91, 11, 120, 209, 114, 220, 77, 67, 64, 236, 17, 236, 17, 236, 17];
        let ecc = remainder(&data, &divisor(10));
        assert_eq!(ecc, vec![196, 35, 39, 119, 235, 215, 231, 226, 93, 23]);
    }
}

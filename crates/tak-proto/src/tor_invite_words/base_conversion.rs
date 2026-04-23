use anyhow::{Result, bail};

pub(super) fn encode_word_indices(bytes: &[u8], output_len: usize) -> Result<Vec<u16>> {
    let mut value = bytes.to_vec();
    let mut indices = vec![0_u16; output_len];
    let base = word_base();

    for index in indices.iter_mut().rev() {
        let (quotient, remainder) = divmod_be(&value, base);
        value = quotient;
        *index = u16::try_from(remainder).expect("word list length should fit in u16");
    }

    if !value.is_empty() {
        bail!("tor invite words overflowed the configured phrase length");
    }
    Ok(indices)
}

pub(super) fn decode_word_indices(indices: &[u16], output_len: usize) -> Result<Vec<u8>> {
    let mut bytes = vec![0_u8; output_len];
    let base = word_base();
    for &index in indices {
        mul_add_be(&mut bytes, base, u32::from(index))?;
    }
    Ok(bytes)
}

fn word_base() -> u32 {
    u32::try_from(super::dictionary::word_list().len()).expect("word list length should fit in u32")
}

fn divmod_be(value: &[u8], divisor: u32) -> (Vec<u8>, u32) {
    let mut quotient = Vec::with_capacity(value.len());
    let mut remainder = 0_u32;
    let mut seen_non_zero = false;

    for &byte in value {
        let partial = (remainder << 8) | u32::from(byte);
        let digit = partial / divisor;
        remainder = partial % divisor;
        if digit != 0 || seen_non_zero {
            quotient.push(digit as u8);
            seen_non_zero = true;
        }
    }

    (quotient, remainder)
}

fn mul_add_be(value: &mut [u8], multiplier: u32, addend: u32) -> Result<()> {
    let mut carry = u64::from(addend);
    for byte in value.iter_mut().rev() {
        let product = u64::from(*byte) * u64::from(multiplier) + carry;
        *byte = (product & 0xff) as u8;
        carry = product >> 8;
    }
    if carry != 0 {
        bail!("tor invite words overflow 35 bytes");
    }
    Ok(())
}

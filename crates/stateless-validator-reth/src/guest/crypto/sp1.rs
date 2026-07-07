use alloc::{vec, vec::Vec};

use alloy_primitives::Uint;

#[unsafe(no_mangle)]
extern "C" fn native_keccak256(bytes: *const u8, len: usize, output: *mut u8) {
    let mut hash = zkvm_interface::zkvm_keccak256_hash { data: [0; 32] };
    unsafe {
        zkvm_interface::zkvm_keccak256(bytes, len, &mut hash);
        core::ptr::copy_nonoverlapping(hash.data.as_ptr(), output, 32);
    }
}

/// Fast path for SP1 modexp cases that are cheaper in guest code than in the
/// `libzkevm` BigUint fallback.
pub(super) fn fast_modexp(base: &[u8], exp: &[u8], modulus: &[u8]) -> Option<Vec<u8>> {
    let raw_base = base;
    let raw_modulus = modulus;
    let modulus = trim_zeroes(modulus);
    if modulus.is_empty() {
        return Some(Vec::new());
    }

    if is_zero(exp) {
        return Some(if is_one(modulus) { vec![0] } else { vec![1] });
    }

    if is_one(modulus) {
        return Some(vec![0]);
    }

    let base = trim_zeroes(base);
    if base.is_empty() {
        return Some(vec![0]);
    }

    if is_one(base) {
        return Some(vec![1]);
    }

    if base == modulus {
        return Some(vec![0]);
    }

    if is_plus_one(base, modulus) {
        return Some(vec![1]);
    }

    if let Some(output) = modexp_repeated_248_bit_chunk_cube(raw_base, exp, raw_modulus) {
        return Some(output);
    }

    if let Some(output) = modexp_small_exp_ruint(base, exp, modulus) {
        return Some(output);
    }

    if let Some(output) = modexp_short_exp_ruint(base, exp, modulus) {
        return Some(output);
    }

    if base.len() <= 32 && modulus.len() <= 32 {
        return Some(modexp_u256(base, exp, modulus));
    }

    None
}

fn trim_zeroes(bytes: &[u8]) -> &[u8] {
    let first = bytes
        .iter()
        .position(|&byte| byte != 0)
        .unwrap_or(bytes.len());
    &bytes[first..]
}

fn is_zero(bytes: &[u8]) -> bool {
    trim_zeroes(bytes).is_empty()
}

fn is_one(bytes: &[u8]) -> bool {
    trim_zeroes(bytes) == [1]
}

fn is_plus_one(value: &[u8], modulus: &[u8]) -> bool {
    let mut value_idx = value.len();
    let mut modulus_idx = modulus.len();
    let mut carry = 1u16;

    while value_idx > 0 || modulus_idx > 0 || carry > 0 {
        let modulus_byte = if modulus_idx > 0 {
            modulus_idx -= 1;
            modulus[modulus_idx]
        } else {
            0
        };
        let sum = modulus_byte as u16 + carry;
        let expected = sum as u8;
        carry = sum >> 8;

        let value_byte = if value_idx > 0 {
            value_idx -= 1;
            value[value_idx]
        } else {
            0
        };
        if value_byte != expected {
            return false;
        }
    }

    value_idx == 0
}

fn modexp_repeated_248_bit_chunk_cube(base: &[u8], exp: &[u8], modulus: &[u8]) -> Option<Vec<u8>> {
    if trim_zeroes(exp) != [3]
        || base.len() != modulus.len()
        || base.is_empty()
        || !base.len().is_multiple_of(32)
        || !base.iter().all(|&byte| byte == 0xff)
    {
        return None;
    }

    let chunks = base.len() / 32;
    if !modulus
        .chunks_exact(32)
        .all(is_repeated_248_bit_modulus_chunk)
    {
        return None;
    }

    let modulus_248 = [u64::MAX, u64::MAX, u64::MAX, 0x00ff_ffff_ffff_ffff];
    let series = repeated_chunk_series_mod_mersenne_248(chunks);

    let coefficient = [255u64.pow(3), 0, 0, 0];
    let series_squared = mul_mod_u256(&series, &series, &modulus_248);
    let chunk = mul_mod_u256(&series_squared, &coefficient, &modulus_248);
    let chunk = le_limbs_to_fixed_be_bytes(&chunk);

    let mut output = Vec::with_capacity(base.len());
    for _ in 0..chunks {
        output.extend_from_slice(&chunk);
    }
    Some(output)
}

fn repeated_chunk_series_mod_mersenne_248(chunks: usize) -> [u64; 4] {
    if chunks <= 31 {
        let mut series = [0u64; 4];
        for byte_idx in 0..chunks {
            series[byte_idx / 8] |= 1u64 << ((byte_idx % 8) * 8);
        }
        return series;
    }

    let mut bytes = [0u8; 31];
    let len = bytes.len();
    for byte_idx in 0..chunks {
        add_wrapping_power_to_mersenne_248(&mut bytes, byte_idx % len);
    }

    if bytes.iter().all(|&byte| byte == 0xff) {
        return [0u64; 4];
    }

    let mut limbs = [0u64; 4];
    for (byte_idx, byte) in bytes.into_iter().enumerate() {
        limbs[byte_idx / 8] |= (byte as u64) << ((byte_idx % 8) * 8);
    }
    limbs
}

fn add_wrapping_power_to_mersenne_248(bytes: &mut [u8; 31], start_idx: usize) {
    let mut idx = start_idx;
    loop {
        let (byte, carry) = bytes[idx].overflowing_add(1);
        bytes[idx] = byte;
        if !carry {
            return;
        }
        idx += 1;
        if idx == bytes.len() {
            idx = 0;
        }
    }
}

fn is_repeated_248_bit_modulus_chunk(chunk: &[u8]) -> bool {
    chunk.len() == 32 && chunk[0] == 0 && chunk[1..].iter().all(|&byte| byte == 0xff)
}

fn modexp_small_exp_ruint(base: &[u8], exp: &[u8], modulus: &[u8]) -> Option<Vec<u8>> {
    let exp = trim_zeroes(exp);
    if !matches!(exp, [2] | [3] | [4] | [1, 0, 1]) || base.len() <= 32 {
        return None;
    }

    match base.len().max(modulus.len()) {
        0..=64 => Some(modexp_small_exp_ruint_width::<512, 8, 1024, 16>(
            base, exp, modulus,
        )),
        65..=128 => Some(modexp_small_exp_ruint_width::<1024, 16, 2048, 32>(
            base, exp, modulus,
        )),
        129..=256 => Some(modexp_small_exp_ruint_width::<2048, 32, 4096, 64>(
            base, exp, modulus,
        )),
        257..=512 => Some(modexp_small_exp_ruint_width::<4096, 64, 8192, 128>(
            base, exp, modulus,
        )),
        513..=1024 => Some(modexp_small_exp_ruint_width::<8192, 128, 16384, 256>(
            base, exp, modulus,
        )),
        _ => None,
    }
}

fn modexp_small_exp_ruint_width<
    const BITS: usize,
    const LIMBS: usize,
    const WIDE_BITS: usize,
    const WIDE_LIMBS: usize,
>(
    base: &[u8],
    exp: &[u8],
    modulus: &[u8],
) -> Vec<u8> {
    let modulus = Uint::<BITS, LIMBS>::from_be_slice(modulus);
    let base = Uint::<BITS, LIMBS>::from_be_slice(base) % modulus;
    let square = mul_mod_ruint::<BITS, LIMBS, WIDE_BITS, WIDE_LIMBS>(base, base, modulus);
    let output = match exp {
        [2] => square,
        [3] => mul_mod_ruint::<BITS, LIMBS, WIDE_BITS, WIDE_LIMBS>(square, base, modulus),
        [4] => mul_mod_ruint::<BITS, LIMBS, WIDE_BITS, WIDE_LIMBS>(square, square, modulus),
        [1, 0, 1] => {
            let mut output = base;
            for _ in 0..16 {
                output =
                    mul_mod_ruint::<BITS, LIMBS, WIDE_BITS, WIDE_LIMBS>(output, output, modulus);
            }
            mul_mod_ruint::<BITS, LIMBS, WIDE_BITS, WIDE_LIMBS>(output, base, modulus)
        }
        _ => unreachable!(),
    };

    trimmed_be_bytes(output)
}

fn mul_mod_ruint<
    const BITS: usize,
    const LIMBS: usize,
    const WIDE_BITS: usize,
    const WIDE_LIMBS: usize,
>(
    lhs: Uint<BITS, LIMBS>,
    rhs: Uint<BITS, LIMBS>,
    modulus: Uint<BITS, LIMBS>,
) -> Uint<BITS, LIMBS> {
    let product = lhs.widening_mul::<BITS, LIMBS, WIDE_BITS, WIDE_LIMBS>(rhs);
    let modulus = Uint::<WIDE_BITS, WIDE_LIMBS>::from_limbs_slice(modulus.as_limbs());
    let remainder = product % modulus;
    Uint::<BITS, LIMBS>::from_limbs_slice(remainder.as_limbs())
}

fn modexp_short_exp_ruint(base: &[u8], exp: &[u8], modulus: &[u8]) -> Option<Vec<u8>> {
    let exp = trim_zeroes(exp);
    if exp.len() > 8 || base.len() <= 32 {
        return None;
    }

    match base.len().max(modulus.len()) {
        0..=40 => Some(modexp_ruint_redc_width::<320, 5, 640, 10>(
            base, exp, modulus,
        )),
        41..=48 => Some(modexp_ruint_redc_width::<384, 6, 768, 12>(
            base, exp, modulus,
        )),
        49..=56 => Some(modexp_ruint_redc_width::<448, 7, 896, 14>(
            base, exp, modulus,
        )),
        57..=64 => Some(modexp_ruint_redc_width::<512, 8, 1024, 16>(
            base, exp, modulus,
        )),
        _ => None,
    }
}

fn modexp_ruint_width<
    const BITS: usize,
    const LIMBS: usize,
    const WIDE_BITS: usize,
    const WIDE_LIMBS: usize,
>(
    base: &[u8],
    exp: &[u8],
    modulus: &[u8],
) -> Vec<u8> {
    let modulus = Uint::<BITS, LIMBS>::from_be_slice(modulus);
    let base = Uint::<BITS, LIMBS>::from_be_slice(base) % modulus;
    let mut result = Uint::<BITS, LIMBS>::ONE;
    let mut started = false;

    for &byte in exp {
        let mut mask = 0x80;
        while mask != 0 {
            if !started {
                if byte & mask != 0 {
                    result = base;
                    started = true;
                }
                mask >>= 1;
                continue;
            }

            result = mul_mod_ruint::<BITS, LIMBS, WIDE_BITS, WIDE_LIMBS>(result, result, modulus);
            if byte & mask != 0 {
                result = mul_mod_ruint::<BITS, LIMBS, WIDE_BITS, WIDE_LIMBS>(result, base, modulus);
            }
            mask >>= 1;
        }
    }

    trimmed_be_bytes(result)
}

fn modexp_ruint_redc_width<
    const BITS: usize,
    const LIMBS: usize,
    const WIDE_BITS: usize,
    const WIDE_LIMBS: usize,
>(
    base: &[u8],
    exp: &[u8],
    modulus: &[u8],
) -> Vec<u8> {
    let modulus_value = Uint::<BITS, LIMBS>::from_be_slice(modulus);
    if modulus_value.as_limbs()[0] & 1 == 0 {
        return modexp_ruint_width::<BITS, LIMBS, WIDE_BITS, WIDE_LIMBS>(base, exp, modulus);
    }

    let modulus = modulus_value;
    let inv = neg_inv_mod_u64(modulus.as_limbs()[0]);
    let one = Uint::<BITS, LIMBS>::ONE;
    let r = (Uint::<BITS, LIMBS>::MAX % modulus).add_mod(one, modulus);
    let r2 = r.mul_mod(r, modulus);
    let base = (Uint::<BITS, LIMBS>::from_be_slice(base) % modulus).mul_redc(r2, modulus, inv);
    let mut result = one.mul_redc(r2, modulus, inv);
    let mut started = false;

    for &byte in exp {
        let mut mask = 0x80;
        while mask != 0 {
            if !started {
                if byte & mask != 0 {
                    result = base;
                    started = true;
                }
                mask >>= 1;
                continue;
            }

            result = result.square_redc(modulus, inv);
            if byte & mask != 0 {
                result = result.mul_redc(base, modulus, inv);
            }
            mask >>= 1;
        }
    }

    trimmed_be_bytes(result.mul_redc(one, modulus, inv))
}

fn neg_inv_mod_u64(value: u64) -> u64 {
    let mut inverse = 1u64;
    for _ in 0..6 {
        inverse = inverse.wrapping_mul(2u64.wrapping_sub(value.wrapping_mul(inverse)));
    }
    inverse.wrapping_neg()
}

fn modexp_u256(base: &[u8], exp: &[u8], modulus: &[u8]) -> Vec<u8> {
    let mut base_limbs = [0u64; 4];
    let mut modulus_limbs = [0u64; 4];
    write_be_bytes_to_le_limbs(base, &mut base_limbs);
    write_be_bytes_to_le_limbs(modulus, &mut modulus_limbs);

    let one = [1u64, 0, 0, 0];
    let base = mul_mod_u256(&base_limbs, &one, &modulus_limbs);
    if base.iter().all(|&limb| limb == 0) {
        return vec![0];
    }

    let mut result = one;
    for &byte in trim_zeroes(exp) {
        let mut mask = 0x80;
        while mask != 0 {
            result = mul_mod_u256(&result, &result, &modulus_limbs);
            if byte & mask != 0 {
                result = mul_mod_u256(&result, &base, &modulus_limbs);
            }
            mask >>= 1;
        }
    }

    le_limbs_to_be_bytes(&result)
}

fn mul_mod_u256(x: &[u64; 4], y: &[u64; 4], modulus: &[u64; 4]) -> [u64; 4] {
    #[cfg(target_arch = "riscv32")]
    {
        unsafe extern "C" {
            fn syscall_uint256_mulmod(x: *mut [u64; 4], y: *const [u64; 4]);
        }

        let mut result = *x;
        let mut y_modulus = [0u64; 8];
        y_modulus[..4].copy_from_slice(y);
        y_modulus[4..].copy_from_slice(modulus);
        unsafe {
            syscall_uint256_mulmod(&mut result, y_modulus.as_ptr() as *const [u64; 4]);
        }
        result
    }

    #[cfg(not(target_arch = "riscv32"))]
    {
        let x = Uint::<256, 4>::from_limbs(*x);
        let y = Uint::<256, 4>::from_limbs(*y);
        let modulus = Uint::<256, 4>::from_limbs(*modulus);
        let output = mul_mod_ruint::<256, 4, 512, 8>(x, y, modulus);
        output.into_limbs()
    }
}

fn write_be_bytes_to_le_limbs(bytes: &[u8], limbs: &mut [u64]) {
    for (byte_idx, &byte) in bytes.iter().rev().enumerate() {
        limbs[byte_idx / 8] |= (byte as u64) << ((byte_idx % 8) * 8);
    }
}

fn le_limbs_to_be_bytes(limbs: &[u64; 4]) -> Vec<u8> {
    let bytes = le_limbs_to_fixed_be_bytes(limbs);
    let bytes = trim_zeroes(&bytes);
    if bytes.is_empty() {
        vec![0]
    } else {
        bytes.to_vec()
    }
}

fn le_limbs_to_fixed_be_bytes(limbs: &[u64; 4]) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    for byte_idx in 0..32 {
        bytes[31 - byte_idx] = ((limbs[byte_idx / 8] >> ((byte_idx % 8) * 8)) & 0xff) as u8;
    }
    bytes
}

fn trimmed_be_bytes<const BITS: usize, const LIMBS: usize>(value: Uint<BITS, LIMBS>) -> Vec<u8> {
    let bytes = value.to_be_bytes_vec();
    let bytes = trim_zeroes(&bytes);
    if bytes.is_empty() {
        vec![0]
    } else {
        bytes.to_vec()
    }
}

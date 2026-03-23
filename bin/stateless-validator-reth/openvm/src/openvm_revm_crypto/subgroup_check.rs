//! Copied and modified from https://github.com/axiom-crypto/openvm-eth/blob/938d3c0/crates/revm-crypto/src/subgroup_check.rs.
//! 
//! Subgroup membership checks for elliptic curve points.
//!
//! For pairing-based cryptography to be secure, points must lie in the correct
//! prime-order subgroup of the curve. A point that satisfies the curve equation
//! is not necessarily in the correct subgroup — this only holds when the curve's
//! cofactor is 1 (i.e., the curve group itself is prime-order). When the
//! cofactor is greater than 1, the curve group contains additional points
//! outside the prime-order subgroup, and accepting such points can lead to
//! invalid-curve or small-subgroup attacks.
//!
//! ## When is a subgroup check needed?
//!
//! An elliptic curve group of order `n` can be written as `n = h * r`, where
//! `r` is the prime subgroup order used in the cryptographic protocol and `h`
//! is the **cofactor**. If `h = 1`, every point on the curve is in the
//! prime-order subgroup and no additional check is required. If `h > 1`, a
//! dedicated subgroup check is necessary to reject points that lie in a
//! different subgroup of order dividing `h`.
//!
//! ## Cofactors for the supported curves
//!
//! | Curve       | Group | Cofactor | Subgroup check needed? |
//! |-------------|-------|----------|------------------------|
//! | BN254       | G1    | 1        | No                     |
//! | BN254       | G2    | > 1      | Yes                    |
//! | BLS12-381   | G1    | > 1      | Yes                    |
//! | BLS12-381   | G2    | > 1      | Yes                    |
//!
//! ## Assumption
//!
//! All implementations in this module assume that the point has **already been
//! verified to lie on the curve** (i.e., it satisfies the curve equation). This
//! trait only checks the additional condition of subgroup membership.

use openvm_ecc_guest::weierstrass::WeierstrassPoint;

/// Scalar multiplication using simple double-and-add
fn scalar_mul<P: WeierstrassPoint, const CHECK_SETUP: bool>(
    base: &P,
    scalar: impl AsRef<[u64]>,
) -> P {
    let mut result = P::IDENTITY;
    let mut temp = base.clone();
    for limb in scalar.as_ref() {
        for bit_idx in 0..64u32 {
            if (limb >> bit_idx) & 1 == 1 {
                result.add_assign_impl::<CHECK_SETUP>(&temp);
            }
            temp.double_assign_impl::<CHECK_SETUP>();
        }
    }
    result
}

/// Checks whether an elliptic curve point belongs to the correct prime-order
/// subgroup.
///
/// This trait assumes that the point is already known to be on the curve. It
/// only verifies the additional property of subgroup membership, which is
/// necessary when the curve has cofactor greater than 1.
pub(crate) trait SubgroupCheck: WeierstrassPoint {
    /// Returns `true` if this point lies in the correct prime-order subgroup.
    ///
    /// # Assumption
    ///
    /// The caller must ensure that the point satisfies the curve equation
    /// before calling this method. If the point is not on the curve, the
    /// result is meaningless.
    fn is_in_correct_subgroup(&self) -> bool;
}

mod impl_bn {
    use alloy_primitives::hex;
    use openvm_ecc_guest::{algebra::field::FieldExtension, weierstrass::WeierstrassPoint};
    use openvm_pairing::bn254 as bn;

    /// The value `6x²` is the BN254 curve parameter stored as two little-endian `u64` limbs.
    const SIX_X_SQUARED: [u64; 2] = [17887900258952609094, 8020209761171036667];

    /// First Fp2 coefficient of the untwist-Frobenius-twist endomorphism ψ on BN254's
    /// G2 twist curve.
    ///
    /// Ref: [arkworks bn254/g2.rs](https://github.com/arkworks-rs/algebra/blob/master/curves/bn254/src/curves/g2.rs).
    const P_POWER_ENDOMORPHISM_COEFF_0: bn::Fp2 = bn::Fp2::new(
        bn::Fp::from_const_bytes(hex!(
            "3d556f175795e3990c33c3c210c38cb743b159f53cec0b4cf711794f9847b32f"
        )),
        bn::Fp::from_const_bytes(hex!(
            "a2cb0f641cd56516ce9d7c0b1d2aae3294075ad78bcca44b20aeeb6150e5c916"
        )),
    );

    /// Second Fp2 coefficient of the untwist-Frobenius-twist endomorphism ψ on BN254's
    /// G2 twist curve.
    ///
    /// Ref: [arkworks bn254/g2.rs](https://github.com/arkworks-rs/algebra/blob/master/curves/bn254/src/curves/g2.rs).
    const P_POWER_ENDOMORPHISM_COEFF_1: bn::Fp2 = bn::Fp2::new(
        bn::Fp::from_const_bytes(hex!(
            "5a13a071460154dc9859c9a9ede0aadbb9f9e2b698c65edcdcf59a4805f33c06"
        )),
        bn::Fp::from_const_bytes(hex!(
            "e3b02326637fd382d25ba28fc97d80212b6f79eca7b504079a0441acbc3cc007"
        )),
    );

    /// BN254 G1 has cofactor 1, so the curve group is exactly the prime-order
    /// subgroup. Any point that lies on the curve is necessarily in the correct
    /// subgroup, making an explicit check unnecessary.
    impl super::SubgroupCheck for bn::G1Affine {
        fn is_in_correct_subgroup(&self) -> bool {
            true
        }
    }

    /// BN254 G2 is defined over the sextic twist curve, which has cofactor > 1.
    /// A point on the twist curve may not be in the prime-order subgroup.
    ///
    /// Implements section 4.3 of https://eprint.iacr.org/2022/352.pdf to check `[6x²]P == ψ(P)`.
    impl super::SubgroupCheck for bn::G2Affine {
        fn is_in_correct_subgroup(&self) -> bool {
            // 1. Compute [6x²]P using double-and-add.
            //
            // `CHECK_SETUP=false` since `set_up_once` is a no-op, given that bn254::G2Affine is
            // implemented via [`impl_sw_affine`].
            let x_times_point = super::scalar_mul::<_, false>(self, SIX_X_SQUARED);

            // 2. Compute ψ(P), i.e. "untwist-Frobenius-twist".
            //
            // - ψ(P).x = frob(P.x) · COEFF_0
            // - ψ(P).y = frob(P.y) · COEFF_1
            let endomorphism_point = {
                let psi_x = self.x().frobenius_map(1) * P_POWER_ENDOMORPHISM_COEFF_0;
                let psi_y = self.y().frobenius_map(1) * P_POWER_ENDOMORPHISM_COEFF_1;
                Self::from_xy_unchecked(psi_x, psi_y)
            };

            x_times_point.eq(&endomorphism_point)
        }
    }
}

mod impl_bls {
    use std::ops::{MulAssign, Neg};

    use alloy_primitives::hex;
    use openvm_ecc_guest::{algebra::field::FieldExtension, weierstrass::WeierstrassPoint, Group};
    use openvm_pairing::bls12_381 as bls;

    /// The BLS12-381 curve parameter `|u| = 0xd201000000010000`. The parameter `u`
    /// is negative; the sign is applied via explicit `.neg()` in the algorithms.
    const X: [u64; 1] = [0xd201000000010000];

    /// A non-trivial cube root of unity in Fq (`β³ = 1, β ≠ 1`), used for the GLV
    /// endomorphism `σ: (x, y) → (βx, y)` on G1.
    ///
    /// Ref: [arkworks bls12_381/g1.rs](https://github.com/arkworks-rs/algebra/blob/master/curves/bls12_381/src/curves/g1.rs).
    const BETA: bls::Fp = bls::Fp::from_const_bytes(hex!(
        "fefffeffffff012e02000a6213d817de8896f8e63ba9b3ddea770f6a07c669ba51ce76df2f67195f0000000000000000"
    ));

    /// Fp2 coefficient for the untwist-Frobenius-twist endomorphism ψ on BLS12-381's
    /// G2 twist curve. Has `c0 = 0`, which the implementation exploits to replace a
    /// full Fp2 multiplication with two Fp multiplications.
    ///
    /// Ref: [arkworks bls12_381/g2.rs](https://github.com/arkworks-rs/algebra/blob/master/curves/bls12_381/src/curves/g2.rs).
    const P_POWER_ENDOMORPHISM_COEFF_0: bls::Fp2 = bls::Fp2::new(
        bls::Fp::from_const_bytes(hex!(
            "000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"
        )),
        bls::Fp::from_const_bytes(hex!(
            "adaa00000000fd8bfdff494feb2794409b5fb80f65297d89d49a75897d850daa85ded463864002ec99e67f39ea11011a"
        )),
    );

    /// Second Fp2 coefficient for the untwist-Frobenius-twist endomorphism ψ on
    /// BLS12-381's G2 twist curve: `ψ(P).y = frob(P.y) · COEFF_1`.
    ///
    /// Ref: [arkworks bls12_381/g2.rs](https://github.com/arkworks-rs/algebra/blob/master/curves/bls12_381/src/curves/g2.rs).
    const P_POWER_ENDOMORPHISM_COEFF_1: bls::Fp2 = bls::Fp2::new(
        bls::Fp::from_const_bytes(hex!(
            "a2de1b12047beef10afa673ecf6644305eb41ef6896439ef60cfb130d9ed3d1cd92c7ad748c4e9e28ea68001e6035213"
        )),
        bls::Fp::from_const_bytes(hex!(
            "09cce3edfb8410c8f405ec722f9967eec5419200176ef7775e43d3c2ab5d3948fe7fd16b6de331680b40ff37040eaf06"
        )),
    );

    /// BLS12-381 G1 has cofactor > 1, so not every point on the curve is in the
    /// prime-order subgroup.
    ///
    /// Implements section 6 of https://eprint.iacr.org/2021/1130.
    impl super::SubgroupCheck for bls::G1Affine {
        fn is_in_correct_subgroup(&self) -> bool {
            // 1. Compute [x]P using double-and-add.
            //
            // `CHECK_SETUP=true` given that bls12_381::G1Affine is implemented via [`sw_declare`]
            // that does in fact do a setup.
            //
            // If [x]P == P but P != identity then point is not in the right subgroup.
            let x_times_point = super::scalar_mul::<_, true>(self, X);
            if self.eq(&x_times_point) && !self.is_identity() {
                return false;
            }

            // 2. Compute -[x²]P.
            //
            // Here we can assume `CHECK_SETUP=false` since setup has necessarily been done above.
            let minus_x_squared_times_point =
                super::scalar_mul::<_, false>(&x_times_point, X).neg();

            // 2. Compute endomorphism
            //
            // - σ: (x, y) → (βx, y)
            let endomorphism_point = {
                let mut result = self.clone();
                result.x_mut().mul_assign(&BETA);
                result
            };

            minus_x_squared_times_point.eq(&endomorphism_point)
        }
    }

    /// BLS12-381 G2 is defined over the twist curve, which has cofactor > 1.
    /// A point on the twist curve may not be in the prime-order subgroup.
    ///
    /// Implements section 4 of https://eprint.iacr.org/2021/1130.
    impl super::SubgroupCheck for bls::G2Affine {
        fn is_in_correct_subgroup(&self) -> bool {
            // 1. Compute -[x]P using double-and-add (X is negative).
            let x_times_point = super::scalar_mul::<_, true>(self, X).neg();

            // 2. Compute ψ(P)
            let endomorphism_point = {
                let tmp_x = self.x().frobenius_map(1);
                let psi_x_c0 = -P_POWER_ENDOMORPHISM_COEFF_0.c1 * tmp_x.c1;
                let psi_x_c1 = P_POWER_ENDOMORPHISM_COEFF_0.c1 * tmp_x.c0;
                let psi_x = bls::Fp2::new(psi_x_c0, psi_x_c1);
                let psi_y = self.y().frobenius_map(1) * P_POWER_ENDOMORPHISM_COEFF_1;
                Self::from_xy_unchecked(psi_x, psi_y)
            };

            x_times_point.eq(&endomorphism_point)
        }
    }
}
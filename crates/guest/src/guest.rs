//! This mod provides trait for guest program abstraction, that can also be
//! shared between Rust guest and host.

use alloc::vec::Vec;
use core::convert::identity;

use ere_codec::{Decode, Encode};
use ere_platform_core::Platform;
use sha2::{Digest, Sha256};

/// Guest program that can be ran given [`Platform`] implementation.
pub trait Guest {
    /// Input type read from the host, deserialized via [`Decode`].
    type Input: Encode + Decode;

    /// Output type written to the host, serialized via [`Encode`].
    type Output: Encode + Decode;

    /// Executes the core computation logic of the guest program.
    ///
    /// This method takes the deserialized input and produces the output for the guest program.
    /// It is called by [`Guest::run`] after reading and deserializing the input.
    fn compute<P: Platform>(input: Self::Input) -> Self::Output;

    /// Runs the complete guest program workflow: reads input, computes output, and writes output.
    ///
    /// This is the main entry point for executing a guest program. It:
    /// 1. Reads the input with the platform and decodes it via [`Decode::decode_from_slice`]
    /// 2. Calls [`Guest::compute`] to process the input
    /// 3. Encodes the output via [`Encode::encode_to_vec`] and writes it with the platform
    fn run<P: Platform>()
    where
        Self: Sized,
    {
        run_inner::<Self, P, _>(identity);
    }

    /// Runs the complete guest program workflow but hash the output by sha256
    /// before calling the inner `P::write_whole_output`.
    fn run_output_sha256<P: Platform>()
    where
        Self: Sized,
    {
        run_inner::<Self, P, _>(|output_bytes| {
            P::cycle_scope("sha256_output_bytes", || Sha256::digest(&output_bytes))
        });
    }
}

fn run_inner<G: Guest, P: Platform, T: AsRef<[u8]>>(output_bytes_modifier: impl Fn(Vec<u8>) -> T) {
    let input_bytes = P::cycle_scope("read_input", || P::read_whole_input());

    let input = P::cycle_scope("deserialize_input", || {
        G::Input::decode_from_slice(&input_bytes).unwrap()
    });

    let output = G::compute::<P>(input);

    let output_bytes = P::cycle_scope("serialize_output", || output.encode_to_vec().unwrap());

    let modified_output_bytes = output_bytes_modifier(output_bytes);

    P::cycle_scope("write_output", || {
        P::write_whole_output(modified_output_bytes.as_ref())
    });
}

/// Associated type `Input` of [`Guest`].
pub type GuestInput<G> = <G as Guest>::Input;

/// Associated type `Output` of [`Guest`].
pub type GuestOutput<G> = <G as Guest>::Output;

use stateless_validator_common::new_payload_request::Sha256Hasher;

unsafe extern "C" {
    /// Extern function impl https://github.com/openvm-org/openvm/blob/v1.4.3/extensions/sha256/guest/src/lib.rs#L24.
    fn zkvm_sha256_impl(input: *const u8, input_len: usize, output: *mut u8);
}

/// OpenVM SHA-256 provider for SSZ tree hashing.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct OpenVMSha256Hasher;

impl Sha256Hasher for OpenVMSha256Hasher {
    fn hash(&self, input: &[u8]) -> [u8; 32] {
        let mut output = [0u8; 32];
        unsafe {
            zkvm_sha256_impl(input.as_ptr(), input.len(), output.as_mut_ptr());
        }
        output
    }
}

use stateless_validator_common::new_payload_request::Sha256Hasher;

unsafe extern "C" {
    /// Extern function impl https://github.com/0xPolygonHermez/zisk/blob/v0.16.1/ziskos/entrypoint/src/zisklib/lib/sha256.rs#L120.
    fn sha256_c(input: *const u8, input_len: usize, output: *mut u8);
}

/// ZisK SHA-256 provider for SSZ tree hashing.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ZiskSha256Hasher;

impl Sha256Hasher for ZiskSha256Hasher {
    fn hash(&self, input: &[u8]) -> [u8; 32] {
        let mut output = [0u8; 32];
        unsafe {
            sha256_c(input.as_ptr(), input.len(), output.as_mut_ptr());
        }
        output
    }
}

//! Progressive SSZ list support following the current EIP-7916 tree shape.
//!
//! The pinned `libssz-types` release implements an earlier draft whose branch
//! order differs from the current standard and Lighthouse `unstable`.

use alloc::vec::Vec;
use core::ops::Deref;

use libssz::{DecodeError, SszDecode, SszEncode};
use libssz_merkle::{HashTreeRoot, Node, Sha256Hasher, hash_nodes, merkleize, mix_in_length, pack};

/// An unbounded SSZ list whose hash-tree root uses progressive merkleization.
///
/// Its wire encoding is identical to `Vec<T>`. Consensus maximums that used to
/// be encoded in bounded list types must be checked separately at runtime.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProgressiveList<T>(Vec<T>);

impl<T> ProgressiveList<T> {
    /// Creates an empty progressive list.
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// Appends one value.
    pub fn push(&mut self, value: T) {
        self.0.push(value);
    }

    /// Returns the number of values.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns whether the list is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Consumes the list and returns its values.
    pub fn into_inner(self) -> Vec<T> {
        self.0
    }
}

impl<T> Default for ProgressiveList<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Deref for ProgressiveList<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> From<Vec<T>> for ProgressiveList<T> {
    fn from(values: Vec<T>) -> Self {
        Self(values)
    }
}

impl<T> FromIterator<T> for ProgressiveList<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<T> IntoIterator for ProgressiveList<T> {
    type Item = T;
    type IntoIter = alloc::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a ProgressiveList<T> {
    type Item = &'a T;
    type IntoIter = core::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<T: SszEncode> SszEncode for ProgressiveList<T> {
    fn is_fixed_size() -> bool {
        false
    }

    fn fixed_size() -> usize {
        0
    }

    fn encoded_len(&self) -> usize {
        self.0.encoded_len()
    }

    fn ssz_append(&self, buffer: &mut Vec<u8>) {
        self.0.ssz_append(buffer);
    }
}

impl<T: SszDecode> SszDecode for ProgressiveList<T> {
    fn is_fixed_size() -> bool {
        false
    }

    fn fixed_size() -> usize {
        0
    }

    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, DecodeError> {
        Vec::<T>::from_ssz_bytes(bytes).map(Self)
    }
}

impl<T: HashTreeRoot + SszEncode> HashTreeRoot for ProgressiveList<T> {
    fn hash_tree_root(&self, hasher: &impl Sha256Hasher) -> Node {
        let chunks = if T::is_basic_type() {
            let mut serialized = Vec::new();
            for item in &self.0 {
                item.ssz_append(&mut serialized);
            }
            pack(&serialized)
        } else {
            self.0
                .iter()
                .map(|item| item.hash_tree_root(hasher))
                .collect()
        };

        let root = merkleize_progressive(hasher, &chunks);
        mix_in_length(hasher, &root, self.len())
    }
}

/// Merkleizes chunks using the EIP-7916 progressive `1, 4, 16, ...` tree.
pub fn merkleize_progressive(hasher: &impl Sha256Hasher, chunks: &[Node]) -> Node {
    merkleize_progressive_inner(hasher, chunks, 1)
}

fn merkleize_progressive_inner(
    hasher: &impl Sha256Hasher,
    chunks: &[Node],
    num_leaves: usize,
) -> Node {
    if chunks.is_empty() {
        return [0; 32];
    }

    let take = core::cmp::min(num_leaves, chunks.len());
    let subtree = merkleize(hasher, &chunks[..take], Some(num_leaves));
    let remainder =
        merkleize_progressive_inner(hasher, &chunks[take..], num_leaves.saturating_mul(4));
    hash_nodes(hasher, &subtree, &remainder)
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use hex_literal::hex;
    use libssz::{SszDecode, SszEncode};
    use libssz_merkle::{HashTreeRoot, Sha2Hasher};

    use super::ProgressiveList;

    #[test]
    fn nonempty_basic_list_matches_lighthouse_unstable() {
        // Generated with Lighthouse `unstable` at
        // e6a90c168436d8b8d6b5c779c9b0550bd56fb8c7.
        let list = ProgressiveList::from((0_u8..96).collect::<Vec<_>>());

        assert_eq!(
            list.hash_tree_root(&Sha2Hasher),
            hex!("a812257f8075af058ebaa831c2c1361660131a647866c3d0ca56af0df68fb896")
        );
        assert_eq!(
            ProgressiveList::<u8>::from_ssz_bytes(&list.to_ssz()).unwrap(),
            list
        );
    }
}

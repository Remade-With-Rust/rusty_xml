//! Duplicate attributes, for both parsers (#21, #25, #71). A short tag is
//! checked by a pairwise scan of the attributes already accepted; a tag past
//! [`ATTR_SCAN_MAX`] goes through [`WideSeen`], so 100,000 attributes cost a
//! hash each instead of a scan each.

use std::collections::HashSet;
use std::hash::{BuildHasher, Hash};

/// Up to this many accepted attributes the pairwise scan is cheaper than any
/// set; past it, it is quadratic. One value for both parsers.
pub(crate) const ATTR_SCAN_MAX: usize = 32;

/// Keyed hashes of the attribute keys seen on one wide tag. It holds hashes,
/// not keys, so it borrows nothing from the attribute list the caller keeps
/// growing; two keys can share a hash, so a hit is only a "maybe", which the
/// caller confirms against the real keys. Names are attacker-chosen, so the
/// set keeps the standard library's keyed hasher.
pub(crate) struct WideSeen(HashSet<u64>);

impl WideSeen {
    pub(crate) fn with_capacity(n: usize) -> Self {
        WideSeen(HashSet::with_capacity(n))
    }

    /// Record `key`; `false` when its hash was already present, meaning the
    /// key may repeat an earlier one and must be confirmed.
    #[inline]
    pub(crate) fn insert<K: Hash>(&mut self, key: K) -> bool {
        let h = self.0.hasher().hash_one(key);
        self.0.insert(h)
    }
}

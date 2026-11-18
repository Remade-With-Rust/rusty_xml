//! One node-arena policy for both parsers (#49, #62, #70, #71): reserve from
//! a count of markup, so the arena rarely regrows and never commits far more
//! than the tree will use, then give the slack back once the tree is built.

use rusty_xml_tree::XmlDoc;

/// Nodes to reserve for `markup` markup characters at `num / den` nodes
/// each. Never more than one node per 2 input bytes, the densest markup gets
/// (`<i>x<i>x`, `<a b c>`), so input made of `<` cannot reserve more than a
/// real tree of its size could fill.
pub(crate) fn reserve(len: usize, markup: usize, num: usize, den: usize) -> usize {
    (markup.saturating_mul(num) / den).min(len / 2)
}

/// Count `<` and `=` in an XML document: elements and text follow the
/// first, attributes the second. `<` is 0x3C and `=` 0x3D, so one compare of
/// `x | 1` against `=` finds both. Fixed 240-byte blocks with no exit inside
/// vectorize, and 240 keeps the per-block count inside a `u8` while summing
/// it a quarter as often as 64-byte blocks: 0.35 -> about 0.2 Ir per byte.
pub(crate) fn count_lt_eq(b: &[u8]) -> usize {
    const BLOCK: usize = 240;
    let mut n = 0usize;
    let mut blocks = b.chunks_exact(BLOCK);
    for blk in &mut blocks {
        let mut k = 0u8;
        for &x in blk {
            k += u8::from((x | 1) == b'=');
        }
        n += usize::from(k);
    }
    for &x in blocks.remainder() {
        n += usize::from((x | 1) == b'=');
    }
    n
}

/// The arena an XML parse reserves: 3/2 nodes per `<` or `=` (#70). On
/// documentation, SVG and generated data, nodes per `<` plus `=` ran 1.0 to
/// 1.45, where bytes per node ran 4 to 154, so `len / 10` overshot by up to
/// 32 MiB of commit on sparse files and undershot dense ones.
pub(crate) fn xml_reserve(input: &[u8]) -> usize {
    reserve(input.len(), count_lt_eq(input), 3, 2)
}

/// A push parse told the document's length (#75): the markup in the bytes
/// seen so far, until there are enough of them to size the arena once for
/// the whole document instead of doubling it chunk by chunk, which on Windows
/// holds the old and the new arena at once.
pub(crate) struct LengthHint {
    len: usize,
    seen: usize,
    markup: usize,
}

impl LengthHint {
    /// Bytes counted before the arena is sized from them, so that a first
    /// chunk of a few bytes does not decide a reservation for megabytes.
    const SAMPLE: usize = 64 << 10;

    pub(crate) fn new(len: usize, buffered: &[u8]) -> Self {
        let mut h = LengthHint { len, seen: 0, markup: 0 };
        h.see(buffered);
        h
    }

    pub(crate) fn see(&mut self, b: &[u8]) {
        self.seen += b.len();
        self.markup += count_lt_eq(b);
    }

    /// Enough bytes have been counted, or no more are coming.
    pub(crate) fn ready(&self, terminate: bool) -> bool {
        terminate || self.seen >= Self::SAMPLE
    }

    /// [`xml_reserve`]'s rule over the markup the bytes seen project to the
    /// whole length. Not capped at 32 MiB as an estimate from a prefix is:
    /// the caller vouched for the length.
    pub(crate) fn nodes(&self) -> usize {
        let len = self.len.max(self.seen);
        let markup = self.markup as u128 * len as u128 / self.seen.max(1) as u128;
        reserve(len, usize::try_from(markup).unwrap_or(usize::MAX), 3, 2)
    }
}

/// The tree is complete: release the capacity the reserve over-estimated, so
/// the held tree pays for its nodes only. Every parser that returns a tree
/// ends with this, the push parser included (#70).
pub(crate) fn finish(doc: &mut XmlDoc) {
    doc.shrink_nodes();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_across_blocks_and_the_remainder() {
        for len in [0usize, 1, 63, 64, 65, 200, 239, 240, 241, 1000] {
            let b: Vec<u8> = (0..len)
                .map(|i| match i % 5 {
                    0 => b'<',
                    3 => b'=',
                    _ => b'a',
                })
                .collect();
            let want = b.iter().filter(|&&c| c == b'<' || c == b'=').count();
            assert_eq!(count_lt_eq(&b), want, "len {len}");
        }
        let all = vec![b'='; 240 * 3 + 5];
        assert_eq!(count_lt_eq(&all), all.len());
        let near: Vec<u8> = (0u8..=255).collect();
        assert_eq!(count_lt_eq(&near), 2, "only 0x3C and 0x3D count");
        let all = vec![b'<'; 1000];
        assert_eq!(count_lt_eq(&all), all.len());
    }

    #[test]
    fn a_length_hint_projects_the_markup_seen() {
        let doc = b"<a x='1'><b/>text<b/></a>".repeat(100);
        let whole = LengthHint::new(doc.len(), &doc);
        assert!(!whole.ready(false) && whole.ready(true), "2.5 KB is below the sample");
        assert_eq!(whole.nodes(), xml_reserve(&doc), "all of it seen: the one-shot rule");

        let mut half = LengthHint::new(2 * doc.len(), b"");
        half.see(&doc);
        assert_eq!(half.nodes(), 2 * xml_reserve(&doc));

        let short = LengthHint::new(10, &doc);
        assert_eq!(short.nodes(), xml_reserve(&doc), "a length below what arrived");

        let dense = LengthHint::new(usize::MAX, &[b'<'; 64]);
        assert_eq!(dense.nodes(), usize::MAX / 2, "no overflow, one node per 2 bytes");
        assert!(LengthHint::new(usize::MAX, &doc).nodes() < usize::MAX / 2);
        assert_eq!(LengthHint::new(1 << 20, b"").nodes(), 0, "nothing seen");
    }

    #[test]
    fn reserve_is_bounded_by_half_the_input() {
        assert_eq!(reserve(1000, 100, 3, 2), 150);
        assert_eq!(reserve(100, 1000, 3, 2), 50);
        assert_eq!(reserve(10, usize::MAX, 3, 2), 5);
    }
}

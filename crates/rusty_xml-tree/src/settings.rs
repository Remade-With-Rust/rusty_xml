//! One settings type for every long-running call (#71).
//!
//! The HTML and XML parsers, the push and SAX entries, DTD loading, XInclude
//! and the reader all take a [`ParseSettings`]: the interrupt they poll, the
//! nesting cap and the entity amplification factor. The parser crate exports
//! it under its older names as well (`XmlParseSettings`, `HtmlParseSettings`),
//! so code written against either keeps compiling.

use crate::{Interrupt, InterruptPoll};

/// libxml2's default entity amplification factor (`maxAmpl`).
pub const PARSE_MAX_AMPLIFICATION_DEFAULT: u32 = 5;

/// Settings for a long-running call beyond its option bits.
///
/// Build it from [`Default`] (or [`ParseSettings::new`]) and the `with_*`
/// methods; the struct is `#[non_exhaustive]` so later settings can be added
/// without a break. A field a call does not use is ignored: the HTML parser
/// has no entities, so it ignores [`ParseSettings::max_amplification`].
#[derive(Clone, Copy)]
#[non_exhaustive]
pub struct ParseSettings<'a> {
    /// `xmlCtxtSetMaxAmplification`: the entity amplification factor, default
    /// [`PARSE_MAX_AMPLIFICATION_DEFAULT`]. Once entity expansion passes
    /// 1,000,000 bytes, the parse fails if the expanded size divided by this
    /// factor exceeds the input consumed so far. 0 is treated as 1. Raise it
    /// only for trusted input.
    pub max_amplification: u32,
    /// The nesting cap.
    ///
    /// - `0` (the default): the parser's own. HTML: libxml2's 256, or 2048
    ///   under `XML_PARSE_HUGE`. XML: 5,000, or 1,000,000 under
    ///   `XML_PARSE_HUGE`.
    /// - A positive value: that cap.
    /// - A negative value (`-1`): no cap.
    pub max_depth: i32,
    /// Cooperative cancellation: polled during the call; when it fires the
    /// call fails with its "interrupted" error (`XML_ERR_USER_STOP`, 111, for
    /// the parsers), even under `XML_PARSE_RECOVER`.
    pub interrupt: Option<&'a dyn Interrupt>,
}

impl Default for ParseSettings<'_> {
    fn default() -> Self {
        Self {
            max_amplification: PARSE_MAX_AMPLIFICATION_DEFAULT,
            max_depth: 0,
            interrupt: None,
        }
    }
}

impl std::fmt::Debug for ParseSettings<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParseSettings")
            .field("max_amplification", &self.max_amplification)
            .field("max_depth", &self.max_depth)
            .field("interrupt", &self.interrupt.is_some())
            .finish()
    }
}

impl<'a> ParseSettings<'a> {
    /// The defaults: amplification factor 5, the parser's nesting cap, no
    /// interrupt.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the entity amplification factor (`xmlCtxtSetMaxAmplification`).
    pub fn with_max_amplification(mut self, max_amplification: u32) -> Self {
        self.max_amplification = max_amplification;
        self
    }

    /// Sets the nesting cap: `0` for the parser's own, a positive cap, or
    /// `-1` for none.
    pub fn with_max_depth(mut self, max_depth: i32) -> Self {
        self.max_depth = max_depth;
        self
    }

    /// Sets the interrupt the call polls.
    pub fn with_interrupt(mut self, interrupt: &'a dyn Interrupt) -> Self {
        self.interrupt = Some(interrupt);
        self
    }

    /// The nesting cap these settings give a parser whose own cap is
    /// `default`, or `huge_default` when `huge` (`XML_PARSE_HUGE`) is set.
    /// `usize::MAX` means no cap. One rule for both parsers (#71).
    pub fn depth_cap(&self, default: usize, huge_default: usize, huge: bool) -> usize {
        match self.max_depth {
            0 if huge => huge_default,
            0 => default,
            d if d < 0 => usize::MAX,
            d => d as usize,
        }
    }

    /// An amortized poll of the interrupt, if there is one.
    pub fn poll(&self) -> Option<InterruptPoll<'a>> {
        self.interrupt.map(InterruptPoll::new)
    }

    /// Whether the interrupt has fired, polled now.
    pub fn is_interrupted(&self) -> bool {
        self.interrupt.is_some_and(|i| i.is_interrupted())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn depth_cap_follows_the_documented_rule() {
        let s = ParseSettings::new();
        assert_eq!(s.depth_cap(256, 2048, false), 256);
        assert_eq!(s.depth_cap(256, 2048, true), 2048);
        assert_eq!(s.with_max_depth(10).depth_cap(256, 2048, true), 10);
        assert_eq!(s.with_max_depth(-1).depth_cap(256, 2048, false), usize::MAX);
    }

    #[test]
    fn interrupt_is_seen() {
        let f = AtomicBool::new(false);
        let s = ParseSettings::new().with_interrupt(&f);
        assert!(!s.is_interrupted());
        f.store(true, Ordering::Relaxed);
        assert!(s.is_interrupted());
        assert!(s.poll().unwrap().check().is_err());
        assert!(ParseSettings::new().poll().is_none());
    }
}

//! Cooperative cancellation for long-running calls (#39).
//!
//! libxml2 has no general cancellation: `xmlStopParser` only works from a SAX
//! callback, and the XPath `opLimit` only bounds XPath. A host that wants to
//! stop a parse, an XPath evaluation, a validation, a canonicalization or a
//! save from another thread (a .NET `CancellationToken` behind an FFI layer,
//! or a Rust pipeline with its own deadline) passes an [`Interrupt`] to the
//! `*_interruptible` variant of the call. The call polls it every so often
//! and returns its crate's "interrupted" error once it fires.
//!
//! Rules every interruptible call keeps:
//!
//! - An interrupt never changes a branch: a call that is not interrupted
//!   returns exactly what the plain call returns.
//! - The plain calls keep their signatures and their cost.
//! - There is no clock inside the library. A caller that wants a deadline
//!   checks its own clock in a closure.
//!
//! ```
//! use std::sync::atomic::{AtomicBool, Ordering};
//! use rusty_xml_tree::Interrupt;
//!
//! let flag = AtomicBool::new(false);
//! assert!(!flag.is_interrupted());
//! flag.store(true, Ordering::Relaxed);
//! assert!(flag.is_interrupted());
//!
//! let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
//! let by_clock = move || std::time::Instant::now() >= deadline;
//! assert!(!by_clock.is_interrupted());
//! ```

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// A flag a long-running call polls to learn that it should stop.
///
/// Implemented for [`AtomicBool`], `Arc<AtomicBool>`, any `Fn() -> bool`
/// closure, and [`NeverInterrupt`]. Implementations must be cheap: a call
/// polls it every few hundred units of work.
pub trait Interrupt {
    /// `true` once the call should stop.
    fn is_interrupted(&self) -> bool;
}

impl Interrupt for AtomicBool {
    #[inline]
    fn is_interrupted(&self) -> bool {
        self.load(Ordering::Relaxed)
    }
}

impl Interrupt for Arc<AtomicBool> {
    #[inline]
    fn is_interrupted(&self) -> bool {
        self.load(Ordering::Relaxed)
    }
}

impl<F: Fn() -> bool> Interrupt for F {
    #[inline]
    fn is_interrupted(&self) -> bool {
        self()
    }
}

/// An [`Interrupt`] that never fires.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NeverInterrupt;

impl Interrupt for NeverInterrupt {
    #[inline(always)]
    fn is_interrupted(&self) -> bool {
        false
    }
}

/// The error an interruptible call reports when its [`Interrupt`] fired.
///
/// Crates whose errors are richer wrap this or map it to their own code (the
/// parsers report libxml2's `XML_ERR_USER_STOP`, 111).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Interrupted;

impl fmt::Display for Interrupted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("operation interrupted")
    }
}

impl std::error::Error for Interrupted {}

/// Default number of work units between two polls of an [`Interrupt`].
pub const INTERRUPT_POLL_EVERY: u32 = 256;

/// Amortizes [`Interrupt`] polls: [`InterruptPoll::tick`] counts one unit of
/// work and polls the interrupt once every `every` units.
pub struct InterruptPoll<'a> {
    interrupt: &'a dyn Interrupt,
    every: u32,
    left: u32,
}

impl<'a> InterruptPoll<'a> {
    /// Polls `interrupt` once every [`INTERRUPT_POLL_EVERY`] ticks.
    pub fn new(interrupt: &'a dyn Interrupt) -> Self {
        Self::every(interrupt, INTERRUPT_POLL_EVERY)
    }

    /// Polls `interrupt` once every `every` ticks (at least 1).
    pub fn every(interrupt: &'a dyn Interrupt, every: u32) -> Self {
        let every = every.max(1);
        Self {
            interrupt,
            every,
            left: every,
        }
    }

    /// Counts one unit of work; `Err(Interrupted)` if this tick polled the
    /// interrupt and it had fired.
    #[inline]
    pub fn tick(&mut self) -> Result<(), Interrupted> {
        self.left -= 1;
        if self.left == 0 {
            self.left = self.every;
            if self.interrupt.is_interrupted() {
                return Err(Interrupted);
            }
        }
        Ok(())
    }

    /// Polls the interrupt now, whatever the count.
    #[inline]
    pub fn check(&self) -> Result<(), Interrupted> {
        if self.interrupt.is_interrupted() {
            Err(Interrupted)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn poll_is_amortized_and_reports_once_fired() {
        let polls = Cell::new(0u32);
        let fire_at = 3;
        let f = || {
            polls.set(polls.get() + 1);
            polls.get() >= fire_at
        };
        let mut p = InterruptPoll::every(&f, 10);
        let mut ticks = 0;
        while p.tick().is_ok() {
            ticks += 1;
        }
        assert_eq!(polls.get(), 3);
        assert_eq!(ticks, 29);
        assert!(p.check().is_err());
    }

    #[test]
    fn impls() {
        assert!(!NeverInterrupt.is_interrupted());
        let a = Arc::new(AtomicBool::new(false));
        let b = Arc::clone(&a);
        assert!(!a.is_interrupted());
        b.store(true, Ordering::Relaxed);
        assert!(a.is_interrupted());
        let d: &dyn Interrupt = &*a;
        assert!(d.is_interrupted());
        assert_eq!(Interrupted.to_string(), "operation interrupted");
    }
}

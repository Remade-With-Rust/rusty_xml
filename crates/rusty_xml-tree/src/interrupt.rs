//! Cooperative cancellation shared by the parser, XPath, validators and writer.
//! Uses `core` plus `alloc` only.

extern crate alloc;

use alloc::sync::Arc;
use core::fmt;
use core::num::NonZeroU32;
use core::sync::atomic::{AtomicBool, Ordering};

/// A source of stop requests, polled from the thread doing the work.
pub trait Interrupt {
    /// True once the caller wants the running call to stop.
    fn is_interrupted(&self) -> bool;
}

impl Interrupt for AtomicBool {
    fn is_interrupted(&self) -> bool {
        self.load(Ordering::Relaxed)
    }
}

impl<F: Fn() -> bool> Interrupt for F {
    fn is_interrupted(&self) -> bool {
        self()
    }
}

/// An interrupt that never fires.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoInterrupt;

impl Interrupt for NoInterrupt {
    fn is_interrupted(&self) -> bool {
        false
    }
}

impl<T: Interrupt + ?Sized> Interrupt for Arc<T> {
    fn is_interrupted(&self) -> bool {
        (**self).is_interrupted()
    }
}

/// Returned by [`InterruptCheck::check`] once the interrupt has fired.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Interrupted;

impl fmt::Display for Interrupted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("interrupted")
    }
}

impl core::error::Error for Interrupted {}

/// Error type of the `_with_interrupt` variants whose plain function returns a `Result<_, E>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InterruptError<E> {
    Interrupted,
    Other(E),
}

impl<E> From<Interrupted> for InterruptError<E> {
    fn from(_: Interrupted) -> Self {
        InterruptError::Interrupted
    }
}

impl<E: fmt::Display> fmt::Display for InterruptError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InterruptError::Interrupted => f.write_str("interrupted"),
            InterruptError::Other(e) => e.fmt(f),
        }
    }
}

/// Counts units of work and consults an [`Interrupt`] once per interval.
///
/// The first [`check`](Self::check) consults the interrupt. After a stop every later call
/// returns `Err` without consulting it again, so a swallowed error cannot be followed by `Ok`.
pub struct InterruptCheck<'a> {
    interrupt: &'a dyn Interrupt,
    interval: u32,
    remaining: u32,
    stopped: bool,
}

impl InterruptCheck<'_> {
    /// Consultation interval used by [`InterruptCheck::new`], in checks. The worst-case time from a
    /// stop request to the return is this many checks times the cost of the costliest check, about
    /// 5 us in the quadratic XPath case; larger values trade stop latency for fewer consultations.
    pub const DEFAULT_INTERVAL: NonZeroU32 = NonZeroU32::new(4096).unwrap();
}

impl<'a> InterruptCheck<'a> {
    /// Consults `interrupt` every [`DEFAULT_INTERVAL`](Self::DEFAULT_INTERVAL) checks.
    pub fn new(interrupt: &'a dyn Interrupt) -> Self {
        Self::with_interval(interrupt, Self::DEFAULT_INTERVAL)
    }

    /// Consults `interrupt` every `interval` checks; interval 1 consults on every check.
    pub fn with_interval(interrupt: &'a dyn Interrupt, interval: NonZeroU32) -> Self {
        InterruptCheck {
            interrupt,
            interval: interval.get(),
            remaining: 1,
            stopped: false,
        }
    }

    /// Counts one unit of work; `Err` once the interrupt has fired.
    #[inline]
    pub fn check(&mut self) -> Result<(), Interrupted> {
        if self.stopped {
            return Err(Interrupted);
        }
        self.remaining -= 1;
        if self.remaining == 0 {
            return self.consult();
        }
        Ok(())
    }

    /// True once a check has returned `Err`.
    pub fn is_stopped(&self) -> bool {
        self.stopped
    }

    #[cold]
    fn consult(&mut self) -> Result<(), Interrupted> {
        self.remaining = self.interval;
        if self.interrupt.is_interrupted() {
            self.stopped = true;
            return Err(Interrupted);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::sync::Arc;
    use core::cell::Cell;
    use core::num::NonZeroU32;
    use core::sync::atomic::AtomicBool;

    fn interval(n: u32) -> NonZeroU32 {
        NonZeroU32::new(n).unwrap()
    }

    #[test]
    fn first_check_consults() {
        let calls = Cell::new(0u32);
        let probe = || {
            calls.set(calls.get() + 1);
            false
        };
        let mut check = InterruptCheck::with_interval(&probe, interval(3));
        check.check().unwrap();
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn consults_every_interval() {
        let calls = Cell::new(0u32);
        let probe = || {
            calls.set(calls.get() + 1);
            false
        };
        let mut check = InterruptCheck::with_interval(&probe, interval(3));
        for _ in 0..7 {
            check.check().unwrap();
        }
        assert_eq!(calls.get(), 3);
    }

    #[test]
    fn sticky_after_stop() {
        let calls = Cell::new(0u32);
        let probe = || {
            calls.set(calls.get() + 1);
            calls.get() >= 2
        };
        let mut check = InterruptCheck::with_interval(&probe, interval(1));
        assert!(!check.is_stopped());
        assert_eq!(check.check(), Ok(()));
        assert_eq!(check.check(), Err(Interrupted));
        assert_eq!(check.check(), Err(Interrupted));
        assert_eq!(check.check(), Err(Interrupted));
        assert_eq!(calls.get(), 2);
        assert!(check.is_stopped());
    }

    #[test]
    fn atomic_bool_stops() {
        let flag = AtomicBool::new(true);
        let mut check = InterruptCheck::new(&flag);
        assert_eq!(check.check(), Err(Interrupted));
    }

    #[test]
    fn no_interrupt_never_stops() {
        let mut check = InterruptCheck::new(&NoInterrupt);
        for _ in 0..10_000 {
            assert_eq!(check.check(), Ok(()));
        }
    }

    #[test]
    fn arc_dyn_impl() {
        let arc: Arc<dyn Interrupt + Send + Sync> = Arc::new(AtomicBool::new(true));
        assert_eq!(InterruptCheck::new(&*arc).check(), Err(Interrupted));
        assert_eq!(InterruptCheck::new(&arc).check(), Err(Interrupted));
    }

    #[test]
    fn interrupt_error_from() {
        assert_eq!(
            InterruptError::<String>::from(Interrupted),
            InterruptError::Interrupted
        );
    }
}

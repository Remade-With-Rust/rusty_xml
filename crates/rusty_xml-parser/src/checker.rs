//! Compile-time switch between a counting interrupt check and no check at all, so the default
//! parse path carries no counter (the `dyn` shape measured +2.8% Ir on `xml_read_memory`).

use rusty_xml_tree::{InterruptCheck, Interrupted};

pub(crate) trait Checker {
    /// False for `NoCheck`: lets callers skip code that only serves a stop.
    const ACTIVE: bool;
    fn tick(&mut self) -> Result<(), Interrupted>;
    fn is_stopped(&self) -> bool;
}

impl Checker for InterruptCheck<'_> {
    const ACTIVE: bool = true;

    #[inline]
    fn tick(&mut self) -> Result<(), Interrupted> {
        self.check()
    }

    #[inline]
    fn is_stopped(&self) -> bool {
        InterruptCheck::is_stopped(self)
    }
}

pub(crate) struct NoCheck;

impl Checker for NoCheck {
    const ACTIVE: bool = false;

    #[inline(always)]
    fn tick(&mut self) -> Result<(), Interrupted> {
        Ok(())
    }

    #[inline(always)]
    fn is_stopped(&self) -> bool {
        false
    }
}

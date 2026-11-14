//! Compile-time switch between a counting interrupt check and none, so `xml_save_doc` and
//! `xml_node_dump` carry no counter (the same shape as `rusty_xml-parser`'s private `checker`).

use rusty_xml_tree::{InterruptCheck, Interrupted};

pub(crate) trait Checker {
    fn tick(&mut self) -> Result<(), Interrupted>;
}

impl Checker for InterruptCheck<'_> {
    #[inline]
    fn tick(&mut self) -> Result<(), Interrupted> {
        self.check()
    }
}

pub(crate) struct NoCheck;

impl Checker for NoCheck {
    #[inline(always)]
    fn tick(&mut self) -> Result<(), Interrupted> {
        Ok(())
    }
}

/// The result of a run that cannot have been stopped, as the plain functions return it.
pub(crate) fn plain<T>(r: Result<T, Interrupted>) -> T {
    match r {
        Ok(value) => value,
        Err(Interrupted) => unreachable!("NoCheck never stops"),
    }
}

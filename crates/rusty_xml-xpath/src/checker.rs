//! Compile-time switch between a counting interrupt check and none, so `xml_xpath_eval` carries
//! no counter (the same shape as `rusty_xml-parser`'s private `checker` module).

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

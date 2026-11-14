//! Compile-time switch between a counting interrupt check and none, so the plain validators carry
//! no counter (the same shape as `rusty_xml-parser`'s private `checker` module).

use rusty_xml_tree::{InterruptCheck, InterruptError, Interrupted};

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

/// A validation failure that is not a stop: the validators' `String` errors, wrapped.
pub(crate) fn other(message: impl Into<String>) -> InterruptError<String> {
    InterruptError::Other(message.into())
}

/// The result of a run that cannot have been stopped, as the plain functions return it.
pub(crate) fn plain<T>(r: Result<T, InterruptError<String>>) -> Result<T, String> {
    match r {
        Ok(value) => Ok(value),
        Err(InterruptError::Other(message)) => Err(message),
        Err(InterruptError::Interrupted) => unreachable!("NoCheck never stops"),
    }
}

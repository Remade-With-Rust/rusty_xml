//! Interruptible validation and canonicalization (#39).
//!
//! Every long-running entry point of this crate has an `*_interruptible`
//! twin that takes a `&dyn Interrupt`, polls it every few hundred units of
//! work (an element visited, a content model step, a Schematron test), and
//! returns [`ValidError::Interrupted`] once it fires. An interrupt that does
//! not fire changes nothing: the twin returns exactly what the plain call
//! returns, with the plain call's message in [`ValidError::Failed`].
//!
//! The plain calls run the same code with a poll that compiles to nothing,
//! so they keep their signatures and their cost.

use rusty_xml_tree::{InterruptPoll, Interrupted};
use std::fmt;

/// The error of an interruptible validator or canonicalizer call.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValidError {
    /// What the plain call reports as `Err(String)`: the document is not
    /// valid, the schema could not be read, and so on.
    Failed(String),
    /// The [`Interrupt`](rusty_xml_tree::Interrupt) passed to the call fired
    /// before the call finished.
    Interrupted,
}

impl ValidError {
    /// `true` when the call stopped because its interrupt fired.
    pub fn is_interrupted(&self) -> bool {
        matches!(self, ValidError::Interrupted)
    }

    /// The message the plain call would have returned.
    pub fn into_message(self) -> String {
        match self {
            ValidError::Failed(s) => s,
            ValidError::Interrupted => Interrupted.to_string(),
        }
    }
}

impl fmt::Display for ValidError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidError::Failed(s) => f.write_str(s),
            ValidError::Interrupted => fmt::Display::fmt(&Interrupted, f),
        }
    }
}

impl std::error::Error for ValidError {}

impl From<String> for ValidError {
    fn from(s: String) -> Self {
        ValidError::Failed(s)
    }
}

impl From<&str> for ValidError {
    fn from(s: &str) -> Self {
        ValidError::Failed(s.to_string())
    }
}

impl From<Interrupted> for ValidError {
    fn from(_: Interrupted) -> Self {
        ValidError::Interrupted
    }
}

/// What the shared code polls. The plain calls pass [`NoPoll`], whose methods
/// are empty and inline away; the interruptible ones pass an
/// [`InterruptPoll`].
pub(crate) trait Poll {
    fn tick(&mut self) -> Result<(), Interrupted>;
    fn check(&self) -> Result<(), Interrupted>;
}

pub(crate) struct NoPoll;

impl Poll for NoPoll {
    #[inline(always)]
    fn tick(&mut self) -> Result<(), Interrupted> {
        Ok(())
    }
    #[inline(always)]
    fn check(&self) -> Result<(), Interrupted> {
        Ok(())
    }
}

impl Poll for InterruptPoll<'_> {
    #[inline]
    fn tick(&mut self) -> Result<(), Interrupted> {
        InterruptPoll::tick(self)
    }
    #[inline]
    fn check(&self) -> Result<(), Interrupted> {
        InterruptPoll::check(self)
    }
}

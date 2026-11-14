//! Compile-time switch between a counting interrupt check and none, so the plain validators carry
//! no counter (the same shape as `rusty_xml-parser`'s private `checker` module).
//!
//! The plain path reaches the parser and XPath through their plain functions and the interrupt path
//! through their `_with_interrupt` variants with the caller's check, so one counter and one sticky
//! state cover a whole validation.

use rusty_xml_parser::{
    default_parse_options, xml_read_memory, xml_read_memory_with_interrupt, XML_ERR_USER_STOP,
};
use rusty_xml_tree::{InterruptCheck, InterruptError, Interrupted, XmlDoc};
use rusty_xml_xpath::{
    xml_xpath_eval, xml_xpath_eval_with_interrupt, XPathObject, XmlXPathContext,
};

pub(crate) trait Checker {
    fn tick(&mut self) -> Result<(), Interrupted>;

    /// True once a check has stopped.
    fn is_stopped(&self) -> bool;

    /// Parses a schema document.
    fn read_schema(&mut self, bytes: &[u8]) -> Result<XmlDoc, InterruptError<String>>;

    /// Evaluates an XPath expression.
    fn xpath(
        &mut self,
        expr: &str,
        ctx: &XmlXPathContext<'_>,
    ) -> Result<XPathObject, InterruptError<String>>;
}

impl Checker for InterruptCheck<'_> {
    #[inline]
    fn tick(&mut self) -> Result<(), Interrupted> {
        self.check()
    }

    fn is_stopped(&self) -> bool {
        InterruptCheck::is_stopped(self)
    }

    fn read_schema(&mut self, bytes: &[u8]) -> Result<XmlDoc, InterruptError<String>> {
        xml_read_memory_with_interrupt(bytes, None, None, default_parse_options(), self).map_err(
            |e| {
                if e.code == XML_ERR_USER_STOP {
                    InterruptError::Interrupted
                } else {
                    other(e.to_string())
                }
            },
        )
    }

    fn xpath(
        &mut self,
        expr: &str,
        ctx: &XmlXPathContext<'_>,
    ) -> Result<XPathObject, InterruptError<String>> {
        xml_xpath_eval_with_interrupt(expr, ctx, self)
    }
}

pub(crate) struct NoCheck;

impl Checker for NoCheck {
    #[inline(always)]
    fn tick(&mut self) -> Result<(), Interrupted> {
        Ok(())
    }

    fn is_stopped(&self) -> bool {
        false
    }

    fn read_schema(&mut self, bytes: &[u8]) -> Result<XmlDoc, InterruptError<String>> {
        xml_read_memory(bytes, None, None, default_parse_options())
            .map_err(|e| other(e.to_string()))
    }

    fn xpath(
        &mut self,
        expr: &str,
        ctx: &XmlXPathContext<'_>,
    ) -> Result<XPathObject, InterruptError<String>> {
        xml_xpath_eval(expr, ctx).map_err(other)
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

/// A branch that may fail without failing the match: `Ok(false)` for an ordinary failure, but a
/// stop propagates instead of reading as "this branch did not match".
pub(crate) fn branch_ok(
    r: Result<(), InterruptError<String>>,
) -> Result<bool, InterruptError<String>> {
    match r {
        Ok(()) => Ok(true),
        Err(InterruptError::Other(_)) => Ok(false),
        Err(InterruptError::Interrupted) => Err(InterruptError::Interrupted),
    }
}

/// Backstop: once a check has stopped, the run reports the stop whatever result a branch produced.
pub(crate) fn settle<T>(
    r: Result<T, InterruptError<String>>,
    check: &impl Checker,
) -> Result<T, InterruptError<String>> {
    if check.is_stopped() {
        Err(InterruptError::Interrupted)
    } else {
        r
    }
}

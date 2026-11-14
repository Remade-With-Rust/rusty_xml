//! Allocation count of `xml_xpath_eval` on the default path.

use rusty_xml::{xml_read_memory, xml_xpath_eval, XmlXPathContext};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct Counting;

thread_local! {
    static ALLOCS: Cell<u64> = const { Cell::new(0) };
}

fn count() {
    let _ = ALLOCS.try_with(|a| a.set(a.get() + 1));
}

// SAFETY: every method forwards to `System` with the caller's layout and pointer unchanged.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count();
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOC: Counting = Counting;

/// `//a|//b` on `<r>` with 10 `a` and 10 `b` children, document and context built outside the
/// counted region, in a debug build of `main` at d2c3be8 (`.work/cancel/baseline.md`, union20).
const MAIN_UNION20_ALLOCS: u64 = 57;

/// `sort_unique` runs five times on this expression, each with 2 <= len <= 1024: one extra
/// allocation each for the cached keys.
const SORT_UNIQUE_CALLS: u64 = 5;

#[test]
fn union_allocates_main_count_plus_one_per_sort() {
    let xml = format!("<r>{}{}</r>", "<a/>".repeat(10), "<b/>".repeat(10));
    let doc = xml_read_memory(xml.as_bytes(), None, None, 0).unwrap();
    let ctx = XmlXPathContext::xml_xpath_new_context(&doc);
    ALLOCS.with(|a| a.set(0));
    let result = xml_xpath_eval("//a|//b", &ctx);
    let allocs = ALLOCS.with(Cell::get);
    drop(result);
    assert_eq!(allocs, MAIN_UNION20_ALLOCS + SORT_UNIQUE_CALLS);
}

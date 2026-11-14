//! Allocation count and peak bytes of the default path, per API.
//!
//! `alloc-count XMLLIST HTMLLIST`: for each API, parses the inputs it needs first, resets the
//! counters, runs the API alone over its list, and prints `<api> allocs N peak_bytes N`.
//! `realloc` counts as one allocation. Peak is the largest growth over the bytes live at the reset.

use rusty_xml::{
    html_read_memory, xml_read_memory, xml_xpath_eval, XmlDoc, XmlXPathContext,
};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicIsize, AtomicU64, Ordering::Relaxed};

struct Counting;

static ALLOCS: AtomicU64 = AtomicU64::new(0);
static LIVE: AtomicIsize = AtomicIsize::new(0);
static PEAK: AtomicIsize = AtomicIsize::new(0);

fn grow(bytes: isize) {
    let live = LIVE.fetch_add(bytes, Relaxed) + bytes;
    PEAK.fetch_max(live, Relaxed);
}

// SAFETY: every method forwards to `System` with the caller's layout and pointer unchanged.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Relaxed);
        grow(layout.size() as isize);
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Relaxed);
        grow(layout.size() as isize);
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size() as isize, Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Relaxed);
        grow(new_size as isize - layout.size() as isize);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOC: Counting = Counting;

const XPATH_LIMIT: usize = 50_000;

fn reset() {
    ALLOCS.store(0, Relaxed);
    LIVE.store(0, Relaxed);
    PEAK.store(0, Relaxed);
}

fn report(api: &str) {
    println!(
        "{api} allocs {} peak_bytes {}",
        ALLOCS.load(Relaxed),
        PEAK.load(Relaxed)
    );
}

fn read_list(list: &str) -> Vec<Vec<u8>> {
    std::fs::read_to_string(list)
        .expect("read list")
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|p| std::fs::read(p.trim()).unwrap_or_else(|e| panic!("{p}: {e}")))
        .collect()
}

fn main() {
    let mut args = std::env::args().skip(1);
    let xml = read_list(&args.next().expect("usage: alloc-count XMLLIST HTMLLIST"));
    let html = read_list(&args.next().expect("usage: alloc-count XMLLIST HTMLLIST"));

    reset();
    for b in &xml {
        drop(xml_read_memory(b, None, None, 0));
    }
    report("xml_read_memory");

    let docs: Vec<XmlDoc> = xml
        .iter()
        .filter(|b| b.len() < XPATH_LIMIT)
        .filter_map(|b| xml_read_memory(b, None, None, 0).ok())
        .collect();
    reset();
    for doc in &docs {
        let ctx = XmlXPathContext::xml_xpath_new_context(doc);
        drop(xml_xpath_eval("//*[@*]", &ctx));
    }
    report("xml_xpath_eval");

    reset();
    for b in &html {
        drop(html_read_memory(b, None, None, 0));
    }
    report("html_read_memory");

    let mut union20 = String::from("<r>");
    union20.push_str(&"<a/>".repeat(10));
    union20.push_str(&"<b/>".repeat(10));
    union20.push_str("</r>");
    let doc = xml_read_memory(union20.as_bytes(), None, None, 0).expect("union20 parses");
    let ctx = XmlXPathContext::xml_xpath_new_context(&doc);
    reset();
    drop(xml_xpath_eval("//a|//b", &ctx));
    report("union20");
}

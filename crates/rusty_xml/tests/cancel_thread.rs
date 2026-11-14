mod support;

use rusty_xml::{
    html_read_memory_with_interrupt, xml_read_memory_with_interrupt, InterruptCheck, XmlDoc,
    XmlError, XML_ERR_USER_STOP,
};
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::{mpsc, Arc};
use std::thread;

/// Runs `parse` with an interrupt that signals a channel on its first consultation and then reads
/// a flag; the setter thread sets the flag only after that signal, so the parse cannot finish first.
fn stopped_from_another_thread(
    parse: impl FnOnce(&mut InterruptCheck<'_>) -> Result<XmlDoc, XmlError>,
) -> XmlError {
    let flag = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel::<()>();
    let setter = {
        let flag = Arc::clone(&flag);
        thread::spawn(move || {
            rx.recv().expect("first consultation");
            flag.store(true, Relaxed);
        })
    };
    let first = Cell::new(true);
    let probe = {
        let flag = Arc::clone(&flag);
        move || {
            if first.replace(false) {
                let _ = tx.send(());
            }
            flag.load(Relaxed)
        }
    };
    let result = parse(&mut InterruptCheck::new(&probe));
    setter.join().expect("setter thread");
    result.err().expect("the stop must surface as an error")
}

const LEN: usize = 32 * 1024 * 1024;

#[test]
fn cross_thread_xml() {
    let doc = format!("<r>{}</r>", "<a>x</a>".repeat(LEN / 8));
    let e = stopped_from_another_thread(|c| xml_read_memory_with_interrupt(doc.as_bytes(), None, None, 0, c));
    assert_eq!(e.code, XML_ERR_USER_STOP);
}

#[test]
fn cross_thread_html() {
    let doc = format!("<!DOCTYPE html><body>{}", "<p>x</p>".repeat(LEN / 8));
    let e = stopped_from_another_thread(|c| html_read_memory_with_interrupt(doc.as_bytes(), None, None, 0, c));
    assert_eq!(e.code, XML_ERR_USER_STOP);
}

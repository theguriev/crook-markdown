//! The doors out of the sandbox, and the stubs that stand in for them.
//!
//! A plugin has a handful of imports and no other way to affect anything. They
//! are wrapped here rather than called from the logic, for one reason:
//! everything in this crate except this file then builds and runs on an
//! ordinary machine, so what the plugin *decides* is tested by `cargo test`
//! rather than by installing it and watching a terminal.
//!
//! Off wasm the stubs write down what was asked for, which is what makes "it
//! asked for the block, and then asked for the text to go on the clipboard" a
//! thing a test can assert.

use crook_plugin_api::{Request, to_bytes};

/// How loud a line to the host's log is.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Level {
    /// Something failed.
    Error = 1,
    /// Something is not right but nothing failed.
    Warn = 2,
    /// Worth knowing.
    Info = 3,
}

#[cfg(target_arch = "wasm32")]
mod imports {
    #[link(wasm_import_module = "crook")]
    unsafe extern "C" {
        pub fn contribute(
            slot: *const u8,
            slot_len: usize,
            entry: *const u8,
            entry_len: usize,
            order: i32,
        );
        pub fn register_action(
            name: *const u8,
            name_len: usize,
            title: *const u8,
            title_len: usize,
        );
        pub fn log(level: i32, text: *const u8, len: usize);
        pub fn request(bytes: *const u8, len: usize) -> i32;
    }
}

/// Contributes to a slot the host declares.
pub fn contribute(slot: &str, entry: &str, order: i32) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        imports::contribute(
            slot.as_ptr(),
            slot.len(),
            entry.as_ptr(),
            entry.len(),
            order,
        );
    }
    #[cfg(not(target_arch = "wasm32"))]
    stub::record_contribution(slot, entry, order);
}

/// Offers an action by name, with what a palette should call it.
///
/// `None` is an action that is reachable and not offered — which is what every
/// action here is: an entry of a block's menu is about the block it was opened
/// on, and one run from a command palette would be an entry about nothing.
pub fn register_action(name: &str, title: Option<&str>) {
    let title = title.unwrap_or("");
    #[cfg(target_arch = "wasm32")]
    unsafe {
        imports::register_action(name.as_ptr(), name.len(), title.as_ptr(), title.len());
    }
    #[cfg(not(target_arch = "wasm32"))]
    stub::record_action(name, title);
}

/// Says something in the host's log, under this plugin's name.
pub fn log(level: Level, text: &str) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        imports::log(level as i32, text.as_ptr(), text.len());
    }
    #[cfg(not(target_arch = "wasm32"))]
    stub::record_log(level, text);
}

/// Asks the host to do something, and returns the ticket its answer will
/// carry — or `None` when the host would not take it.
pub fn ask(request: &Request) -> Option<i32> {
    // Encoded on both targets, so that a request the wire cannot carry is a
    // request nothing here believes it made — a test that never encoded one
    // would pass on a shape the host could not read.
    let bytes = to_bytes(request).ok()?;

    #[cfg(target_arch = "wasm32")]
    let ticket = unsafe { imports::request(bytes.as_ptr(), bytes.len()) };
    #[cfg(not(target_arch = "wasm32"))]
    let ticket = stub::record_request(request, bytes.len());

    // Zero is "not asked": too many outstanding, or bytes the host could not
    // read. Either way there is no answer coming, so nothing may wait on one.
    (ticket > 0).then_some(ticket)
}

/// What the imports do when there is no host on the other side of them.
#[cfg(not(target_arch = "wasm32"))]
pub mod stub {
    use std::cell::RefCell;

    use crook_plugin_api::Request;

    use super::Level;

    /// Everything the plugin has asked for since a test last looked.
    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct Asked {
        /// Slots contributed to, as `(slot, entry, order)`.
        pub contributions: Vec<(String, String, i32)>,
        /// Actions offered, as `(name, title)`.
        pub actions: Vec<(String, String)>,
        /// Requests raised, with the ticket each was given.
        pub requests: Vec<(i32, Request)>,
        /// How many bytes each of them encoded to, newest last.
        pub request_bytes: Vec<usize>,
        /// Lines logged.
        pub logs: Vec<(Level, String)>,
    }

    thread_local! {
        static ASKED: RefCell<Asked> = RefCell::new(Asked::default());
        static TICKETS: RefCell<i32> = const { RefCell::new(0) };
    }

    /// What has been asked for, leaving nothing behind.
    pub fn taken() -> Asked {
        ASKED.with(|asked| std::mem::take(&mut *asked.borrow_mut()))
    }

    /// Forgets everything, including the tickets handed out.
    pub fn forget() {
        let _ = taken();
        TICKETS.with(|tickets| *tickets.borrow_mut() = 0);
    }

    pub(super) fn record_contribution(slot: &str, entry: &str, order: i32) {
        ASKED.with(|asked| {
            asked
                .borrow_mut()
                .contributions
                .push((slot.to_owned(), entry.to_owned(), order));
        });
    }

    pub(super) fn record_action(name: &str, title: &str) {
        ASKED.with(|asked| {
            asked
                .borrow_mut()
                .actions
                .push((name.to_owned(), title.to_owned()));
        });
    }

    pub(super) fn record_log(level: Level, text: &str) {
        ASKED.with(|asked| asked.borrow_mut().logs.push((level, text.to_owned())));
    }

    pub(super) fn record_request(request: &Request, bytes: usize) -> i32 {
        let ticket = TICKETS.with(|tickets| {
            let mut tickets = tickets.borrow_mut();
            *tickets += 1;
            *tickets
        });
        ASKED.with(|asked| {
            let mut asked = asked.borrow_mut();
            asked.requests.push((ticket, request.clone()));
            asked.request_bytes.push(bytes);
        });
        ticket
    }
}

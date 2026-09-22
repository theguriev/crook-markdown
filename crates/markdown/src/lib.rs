//! A command and its output, as Markdown ready to paste.
//!
//! Two entries in the menu a block's three dots open: one copies the command
//! and what it printed as a `console` fence, and the other copies the same
//! thing with a heading, the exit status, the directory and the branch around
//! it — which is a bug report somebody can read.
//!
//! It is a plugin [Crook](https://github.com/theguriev/crook) does not carry,
//! and it does what a plugin from a store may do and nothing else: it has no
//! network, no filesystem, no thread and no clock. It describes two rows, it
//! asks for the block a person ran it on, and it asks for text to be put on
//! the clipboard. Both asks are refused unless somebody allowed them on the
//! Plugins page.
//!
//! # What this file is
//!
//! The ABI, and nothing that thinks. Every export the host calls is here, each
//! of them is three lines, and each hands straight over to [`state`] — so the
//! part that decides anything is a plain Rust module `cargo test` runs on an
//! ordinary machine. See [`sys`] for the other half of that trick.
//!
//! # The one global
//!
//! wasm32 is single-threaded and the host calls these one at a time, so the
//! plugin's state is a `static` reached through an [`UnsafeCell`]. That is the
//! ordinary shape for a wasm guest and it is sound for the reason it is
//! ordinary: there is no second thread to race with, and no export below is
//! re-entrant — the host refuses to call in while it is already inside.

use std::cell::UnsafeCell;

use crook_plugin_api::{
    ABI_VERSION, Answer, Capability, Manifest, Node, Render, from_bytes, to_bytes,
};

pub mod format;
pub mod state;
pub mod sys;
pub mod view;

// The plugin's face, and what it looks like, for the Plugins page and the
// Store. Inside the module rather than beside it, for the reason a plugin is
// one file: what says what the plugin is travels with it. Custom sections,
// not data — they cost no memory and no fuel.
crook_plugin_api::icon!("../../../assets/icon.png");

use state::Markdown;

/// A `static` that is only ever touched by one thread, which on wasm32 is
/// every thread there is.
struct Single<T>(UnsafeCell<T>);

// SAFETY: wasm32 has one thread. Off wasm this crate is a library under test,
// where each test makes a `Markdown` of its own rather than using this one.
unsafe impl<T> Sync for Single<T> {}

impl<T> Single<T> {
    /// SAFETY: the caller must not be inside another borrow. Every export
    /// below takes one, does its work and returns, and the host does not call
    /// in while it is already inside.
    #[allow(clippy::mut_from_ref)]
    unsafe fn get(&self) -> &mut T {
        unsafe { &mut *self.0.get() }
    }
}

/// Everything the plugin knows.
static PLUGIN: Single<Option<Markdown>> = Single(UnsafeCell::new(None));

/// The last thing handed back to the host, kept alive until the next one.
///
/// A tree is answered as an offset and a length into this memory, so the bytes
/// have to outlive the call that returned them. Kept rather than leaked
/// because a render happens every frame, and a leak per frame is a plugin that
/// eventually stops fitting in its own memory.
static ANSWER: Single<Vec<u8>> = Single(UnsafeCell::new(Vec::new()));

/// Packs an answer as the host reads it: `(pointer << 32) | length`.
fn hand_back(bytes: Vec<u8>) -> i64 {
    // SAFETY: see `Single::get`.
    let answer = unsafe { ANSWER.get() };
    *answer = bytes;
    ((answer.as_ptr() as u64) << 32 | answer.len() as u64) as i64
}

/// The plugin's own state, made on first use.
fn plugin() -> &'static mut Markdown {
    // SAFETY: see `Single::get`.
    unsafe { PLUGIN.get() }.get_or_insert_with(Markdown::new)
}

/// Which version of the vocabulary this was built against.
///
/// Called before anything else, and a mismatch is a refusal by number rather
/// than a plugin that decodes a shape which means something else now.
#[unsafe(no_mangle)]
pub extern "C" fn crook_abi_version() -> i32 {
    ABI_VERSION as i32
}

/// Somewhere for the host to put a string it is handing over.
///
/// Exact rather than `Vec::with_capacity`, because [`take`] frees it with the
/// same layout and a capacity the allocator rounded up would be a free of the
/// wrong size.
#[unsafe(no_mangle)]
pub extern "C" fn crook_alloc(length: i32) -> i32 {
    let Ok(layout) = std::alloc::Layout::from_size_align(length.max(1) as usize, 1) else {
        return 0;
    };
    // SAFETY: a non-zero size, and a layout built for it.
    unsafe { std::alloc::alloc(layout) as i32 }
}

/// Copies out what the host wrote there, and gives the memory back.
///
/// SAFETY: `pointer` and `length` must be exactly what a previous
/// [`crook_alloc`] answered and what the host wrote into.
unsafe fn take(pointer: i32, length: i32) -> Vec<u8> {
    if pointer <= 0 || length < 0 {
        return Vec::new();
    }
    // SAFETY: the host wrote `length` bytes at `pointer` before calling in.
    let bytes =
        unsafe { std::slice::from_raw_parts(pointer as *const u8, length as usize) }.to_vec();
    // SAFETY: the same layout `crook_alloc` used.
    unsafe {
        std::alloc::dealloc(
            pointer as *mut u8,
            std::alloc::Layout::from_size_align_unchecked(length.max(1) as usize, 1),
        );
    }
    bytes
}

/// What this plugin is and what it needs to be allowed to do.
///
/// Read before any of it runs, which is what lets a person see what it wants
/// and refuse it without running a line of it.
#[unsafe(no_mangle)]
pub extern "C" fn crook_manifest() -> i64 {
    hand_back(to_bytes(&manifest()).unwrap_or_default())
}

/// The manifest, as a value, so that a test can read it.
pub fn manifest() -> Manifest {
    Manifest {
        abi: ABI_VERSION,
        id: String::from("theguriev/markdown"),
        name: String::from("Markdown"),
        description: String::from("A command and its output, as Markdown ready to paste."),
        version: String::from(env!("CARGO_PKG_VERSION")),
        // Three, and they are the whole of what this does. No network, no
        // files: nothing it copies can leave the machine, and it could not
        // send it anywhere if it wanted to.
        capabilities: vec![
            Capability::ReadBlock,
            Capability::ReadWorkingDirectory,
            Capability::Clipboard,
        ],
    }
}

/// Registers the entries and the actions behind them.
///
/// **From nothing, every time.** A build is not resumed: the host builds a
/// plugin again when it is switched back on and when a person answers what it
/// asked to be allowed, and a ticket left over from the previous life is one
/// nobody will ever answer.
#[unsafe(no_mangle)]
pub extern "C" fn crook_build() -> i32 {
    // SAFETY: see `Single::get`.
    let held = unsafe { PLUGIN.get() };
    *held = Some(Markdown::new());
    held.get_or_insert_with(Markdown::new).build();
    0
}

/// What to draw, and what it is being drawn for.
///
/// The render carries a [`Render`] rather than a slot name, and for this
/// plugin that *is* the feature: its subject is the command whose menu is
/// open, so what it draws — and what a press a moment later is about — comes
/// in through here.
#[unsafe(no_mangle)]
pub extern "C" fn crook_render(render: i32, length: i32) -> i64 {
    // SAFETY: the host allocated and wrote this before calling in.
    let bytes = unsafe { take(render, length) };
    let tree = match from_bytes::<Render>(&bytes) {
        Ok(render) => plugin().render(&render),
        // A render this build cannot read is a host speaking a version this
        // one does not, which the ABI check should already have caught.
        Err(_) => Node::Empty,
    };
    hand_back(to_bytes(&tree).unwrap_or_default())
}

/// Runs one of the actions registered while building.
///
/// The argument is what the thing that was pressed had to say. Both entries
/// here are about the block their menu is open on rather than about anything
/// they were handed, so it is read and ignored — deliberately, and written
/// down here so the next person does not go looking for what it was for.
#[unsafe(no_mangle)]
pub extern "C" fn crook_run(name: i32, length: i32, _argument: i32, _argument_len: i32) -> i32 {
    // SAFETY: as above.
    let name = unsafe { take(name, length) };
    match std::str::from_utf8(&name) {
        Ok(action) => plugin().run(action),
        Err(_) => return 1,
    }
    0
}

/// The answer to something this plugin asked for.
#[unsafe(no_mangle)]
pub extern "C" fn crook_deliver(ticket: i32, bytes: i32, length: i32) -> i32 {
    // SAFETY: as above.
    let bytes = unsafe { take(bytes, length) };
    match from_bytes::<Answer>(&bytes) {
        Ok(answer) => plugin().deliver(ticket, answer),
        // An answer this build cannot read is a host speaking a version this
        // one does not, which the ABI check should already have caught.
        Err(_) => return 1,
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pictures_in_the_module_are_pngs() {
        // A wrong path fails at compile time; a wrong file fails here, on the
        // machine that runs the tests, rather than on the Plugins page.
        assert!(CROOK_ICON.starts_with(b"\x89PNG\r\n\x1a\n"));
    }

    #[test]
    fn the_manifest_says_what_it_needs_in_sentences_a_person_can_refuse() {
        let manifest = manifest();

        assert_eq!(manifest.abi, ABI_VERSION);
        assert_eq!(manifest.id, "theguriev/markdown");
        let sentences: Vec<String> = manifest
            .capabilities
            .iter()
            .map(Capability::sentence)
            .collect();
        assert_eq!(
            sentences,
            vec![
                String::from("Read the command you run it on, and what it printed"),
                String::from("See which project each tab is in"),
                String::from("Read and change your clipboard"),
            ]
        );
    }

    #[test]
    fn what_it_asks_for_is_exactly_what_it_uses() {
        // The one invariant tying the manifest to the code: a plugin that asks
        // for less than it uses is refused at runtime, and one that asks for
        // more is one nobody should allow. This one reads a block and writes a
        // clipboard, and that is all two capabilities cover.
        let keys: Vec<String> = manifest()
            .capabilities
            .iter()
            .flat_map(Capability::keys)
            .collect();

        assert_eq!(
            keys,
            vec![
                String::from("block.read"),
                String::from("cwd.read"),
                String::from("clipboard"),
            ]
        );
    }

    #[test]
    fn the_only_slot_it_draws_in_is_the_one_it_contributed_to() {
        let mut plugin = state::Markdown::new();
        let drawn = plugin.render(&Render {
            slot: String::from(state::SLOT),
            entry: String::from("markdown"),
            subject: None,
        });
        assert_ne!(drawn, Node::Empty);

        let elsewhere = plugin.render(&Render {
            slot: String::from("header.right"),
            entry: String::from("markdown"),
            subject: None,
        });
        assert_eq!(
            elsewhere,
            Node::Empty,
            "asked about a slot it never contributed to, it drew something"
        );
    }
}

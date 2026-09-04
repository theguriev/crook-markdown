//! What the plugin asks to have drawn, which is two labels and nothing else.
//!
//! No padding, no type size, no colour for the row under the pointer: a menu
//! entry is [`Node::Button`] with a label and the action it runs, and the host
//! draws it as a row of *its* menu. That is the whole of the bargain this tier
//! is built on — a plugin that described a row would be a plugin that looks
//! wrong the day the menu changes, in a theme it was never opened in.
//!
//! Both entries are always offered. A plugin describing a menu group is not
//! told which block the menu is open on — that answer belongs to the moment an
//! entry *runs* — so an entry that greyed itself out would be guessing, and a
//! block with nothing in it is handled where it is known about: see
//! [`Markdown::copy`](crate::state::Markdown).

use crook_plugin_api::{Node, Tone};

use crate::state::{FENCED, REPORT};

/// The group this plugin puts in a block's menu.
pub fn menu() -> Node {
    Node::Column(vec![
        Node::Button {
            label: String::from("Copy as Markdown"),
            action: String::from(FENCED),
            tone: Tone::Primary,
        },
        Node::Button {
            label: String::from("Copy as a report"),
            action: String::from(REPORT),
            tone: Tone::Primary,
        },
    ])
}

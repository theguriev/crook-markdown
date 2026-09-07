//! What the plugin asks to have drawn, which is two labels and nothing else.
//!
//! No padding, no type size, no colour for the row under the pointer: an entry
//! is a [`MenuItem`] with a label and the action it runs, and Crook draws it as
//! a row of *its* menu. That is the whole of the bargain this tier is built on
//! — a plugin that described a row would be a plugin that looks wrong the day
//! the menu changes, in a theme it was never opened in.
//!
//! # What the block it is drawn for changes
//!
//! Two things, and both are honesty rather than decoration. An entry that
//! copies a *report* says nothing worth pasting for a command whose shell
//! never reported a status, so on one of those it is not offered — the same
//! rule Crook's own "Copy git branch" follows on a block with no branch behind
//! it. And a plugin that was never granted a sight of the command draws both
//! entries all the same: it will be refused when it asks, and being refused is
//! a sentence a person can act on, where a menu that quietly lost its rows is
//! not.

use crook_plugin_api::{BlockFacts, MenuItem, Node, Subject};

use crate::state::{FENCED, REPORT};

/// The group this plugin puts in a block's menu.
pub fn menu(subject: Option<&Subject>) -> Node {
    let block = match subject {
        Some(Subject::Block(facts)) => Some(facts),
        // A subject this build does not have, or none at all: drawn as though
        // nothing were known, which is what a plugin that guessed would get
        // wrong.
        _ => None,
    };

    let mut items = vec![MenuItem {
        label: String::from("Copy as Markdown"),
        action: String::from(FENCED),
        argument: String::new(),
    }];
    if worth_reporting(block) {
        items.push(MenuItem {
            label: String::from("Copy as a report"),
            action: String::from(REPORT),
            argument: String::new(),
        });
    }

    Node::Menu {
        // Nothing to hang a menu off when the menu *is* the slot. The host
        // draws the items and leaves this alone.
        content: Box::new(Node::Empty),
        items,
    }
}

/// Whether a report would say anything a fence does not.
///
/// A report is the command with its status, its directory and its branch above
/// it. A block that reported none of those is a report with a heading and
/// nothing under it, and an entry that produces one is an entry that wastes a
/// press.
fn worth_reporting(block: Option<&BlockFacts>) -> bool {
    let Some(block) = block else {
        // Drawn for nothing at all, which is a slot with no subject and not a
        // block that said nothing.
        return true;
    };
    match block.ran.as_ref() {
        // Told what ran, so the entry is offered exactly when a report would
        // say more than a fence does.
        Some(ran) => ran.exit.is_some() || block.place.is_some(),
        // Not told, which is a plugin nobody has answered for rather than a
        // command that reported nothing. Offered: the refusal a press earns
        // says what to allow, and a row that quietly vanished says nothing.
        None => true,
    }
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;

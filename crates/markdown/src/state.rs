//! What the plugin does when somebody presses one of its entries.
//!
//! Three steps and no state worth the name: an entry is pressed, the plugin
//! asks the host what block it was pressed on, and when the answer comes back
//! it asks the host to put some text on the clipboard. Everything between the
//! two asks is [`format`](crate::format), which is a plain module with no
//! imports in it.
//!
//! # Why a ticket has to be remembered at all
//!
//! Because the answer arrives later and says nothing about what it was for. A
//! plugin with two entries that both read a block has to know which of them is
//! being answered — otherwise "Copy as a report" pastes a bare fence whenever
//! somebody pressed the other one first.

use crook_plugin_api::{Answer, Request};

use crate::format::{self, Block, Shape};
use crate::sys::{self, Level};

/// What a ticket was asked for.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Want {
    /// The block, to be turned into this shape.
    Block(Shape),
    /// The clipboard, which needs nothing done with its answer beyond knowing
    /// it happened.
    Copy,
}

/// Everything the plugin knows, which is what it is waiting on.
#[derive(Debug, Default)]
pub struct Markdown {
    /// Tickets asked for and not yet answered.
    ///
    /// A list rather than one slot: nothing stops somebody pressing an entry
    /// on one block and another on the next before the first has come back,
    /// and a plugin that kept one ticket would answer the second with the
    /// first one's shape.
    waiting: Vec<(i32, Want)>,
}

/// The name of the entry that copies a fence.
pub const FENCED: &str = "markdown";
/// The name of the entry that copies a report.
pub const REPORT: &str = "report";

/// The slot the entries go in, which is the menu a block's dots open.
pub const SLOT: &str = "block.menu";

/// Where among the menu's other groups this one goes.
///
/// After Crook's own copies and before its scrolls. A plugin's entries are
/// their own group with a rule above them, so this decides which side of that
/// rule the terminal's own entries are on and nothing else.
pub const ORDER: i32 = 15;

impl Markdown {
    /// A plugin that has not been asked for anything yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers the entries and the actions behind them.
    ///
    /// Nothing else happens here. There is no timer, no poll and nothing to
    /// read at startup: this plugin does something exactly when somebody asks
    /// it to, which is what a menu entry is.
    pub fn build(&mut self) {
        self.waiting.clear();
        sys::contribute(SLOT, "markdown", ORDER);
        // Reachable and not offered: an entry of a block's menu is about the
        // block it was opened on, so one run from a command palette would be
        // an entry about nothing.
        sys::register_action(FENCED, None);
        sys::register_action(REPORT, None);
    }

    /// Runs one of them, which is to ask what block it was run on.
    pub fn run(&mut self, action: &str) {
        let shape = match action {
            FENCED => Shape::Fenced,
            REPORT => Shape::Report,
            other => {
                sys::log(Level::Warn, &format!("nothing here is called {other:?}"));
                return;
            }
        };

        // The host answers this with the block whose menu the person used, and
        // only while the entry is running. There is no way to ask for another
        // one and no way to ask later.
        if let Some(ticket) = sys::ask(&Request::Block) {
            self.waiting.push((ticket, Want::Block(shape)));
        }
    }

    /// Takes one answer, and asks for whatever it makes possible.
    pub fn deliver(&mut self, ticket: i32, answer: Answer) {
        let Some(index) = self
            .waiting
            .iter()
            .position(|(waiting, _)| *waiting == ticket)
        else {
            // A ticket nobody here is waiting on. Nothing to do with it, and
            // nothing worth a line in somebody's log.
            return;
        };
        let (_, want) = self.waiting.remove(index);

        // A refusal is not a failure, and the sentence it carries is the one
        // the permission dialog used — so this can say what to go and allow
        // rather than that something went wrong.
        if let Answer::Refused(sentence) = &answer {
            sys::log(
                Level::Info,
                &format!("not allowed to: {sentence}. Allow it on the Plugins page."),
            );
            return;
        }
        if let Answer::Failed(why) = &answer {
            sys::log(Level::Warn, why);
            return;
        }

        match want {
            Want::Block(shape) => self.copy(&answer, shape),
            // The text is on the clipboard. Nothing to say about it: the
            // person watched the menu close and will find out by pasting.
            Want::Copy => {}
        }
    }

    /// Turns a block into text and asks for it to be put on the clipboard.
    fn copy(&mut self, answer: &Answer, shape: Shape) {
        let Some(block) = Block::of(answer) else {
            sys::log(Level::Warn, "that answer was not a block");
            return;
        };
        if block.is_empty() {
            // A shell with no integration leaves a block with neither a
            // command nor any output, and copying an empty fence would
            // quietly replace whatever somebody had on their clipboard.
            sys::log(
                Level::Info,
                "that block has no command and printed nothing, so there is nothing to copy",
            );
            return;
        }

        let text = format::markdown(&block, shape);
        if let Some(ticket) = sys::ask(&Request::Copy { text }) {
            self.waiting.push((ticket, Want::Copy));
        }
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;

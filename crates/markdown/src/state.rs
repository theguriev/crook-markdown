//! What the plugin does when somebody presses one of its entries.
//!
//! Three steps and almost no state: an entry is pressed, the plugin asks what
//! the command printed, and when the answer comes back it asks for the text to
//! go on the clipboard. Everything between the two asks is
//! [`format`](crate::format), which is a plain module with no imports in it.
//!
//! # What is remembered, and why so little
//!
//! The command itself is not asked for. It arrives in the *subject* of every
//! render — Crook draws a block's menu once per frame it is open, and each of
//! those says which command it is about — so what this keeps is the last one it
//! was drawn for. That is sound because a menu is modal: the frame before a
//! press is the block the press is about, and there is no second menu open
//! behind it.
//!
//! What has to be asked for is the output, because it can be a megabyte and a
//! subject is built every frame. That request may only be raised out of a press
//! — which is exactly when this raises it.
//!
//! # Why a ticket has to be remembered at all
//!
//! Because the answer arrives later and says nothing about what it was for. A
//! plugin with two entries that both read a block has to know which of them is
//! being answered — otherwise "Copy as a report" pastes a bare fence whenever
//! somebody pressed the other one first.

use crook_plugin_api::{Answer, BlockFacts, Render, Request, Subject};

use crate::format::{self, Shape};
use crate::sys::{self, Level};

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

/// Everything the plugin knows: the command it was last drawn for, and what it
/// is waiting on.
#[derive(Debug, Default)]
pub struct Markdown {
    /// The block the last render was about, which is the block a press is
    /// about. `None` before the first render and for a build that has just
    /// happened.
    drawn_for: Option<BlockFacts>,
    /// Tickets asked for and not yet answered.
    ///
    /// A list rather than one slot: nothing stops somebody pressing an entry
    /// on one block and another on the next before the first has come back,
    /// and a plugin that kept one ticket would answer the second with the
    /// first one's shape.
    waiting: Vec<(i32, Waiting)>,
}

/// What a ticket was asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Waiting {
    /// Which shape the answer is to be turned into.
    shape: Shape,
    /// The command it is about, as it was when the entry was pressed — not as
    /// it is when the answer lands, because by then the menu has closed and
    /// the plugin is being drawn for nothing.
    block: Option<BlockFacts>,
}

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
        self.drawn_for = None;
        sys::contribute(SLOT, "markdown", ORDER);
        // Reachable and not offered: an entry of a block's menu is about the
        // block it was opened on, so one run from a command palette would be
        // an entry about nothing.
        sys::register_action(FENCED, None);
        sys::register_action(REPORT, None);
    }

    /// What to draw, and what the drawing says about which block this is.
    pub fn render(&mut self, render: &Render) -> crook_plugin_api::Node {
        if render.slot != SLOT {
            // A slot this plugin does not contribute to, which cannot happen
            // and is drawn as nothing rather than guessed at.
            return crook_plugin_api::Node::Empty;
        }

        self.drawn_for = match render.subject.as_ref() {
            Some(Subject::Block(facts)) => Some(facts.clone()),
            _ => None,
        };
        crate::view::menu(render.subject.as_ref())
    }

    /// Runs one of them, which is to ask what the command printed.
    pub fn run(&mut self, action: &str) {
        let shape = match action {
            FENCED => Shape::Fenced,
            REPORT => Shape::Report,
            other => {
                sys::log(Level::Warn, &format!("nothing here is called {other:?}"));
                return;
            }
        };

        // Only a press may ask this, and this is one. The answer is about the
        // block whose menu the person used, which is the block the last render
        // was drawn for.
        if let Some(ticket) = sys::ask(&Request::Output) {
            self.waiting.push((
                ticket,
                Waiting {
                    shape,
                    block: self.drawn_for.clone(),
                },
            ));
        }
    }

    /// Takes one answer, and asks for whatever it makes possible.
    pub fn deliver(&mut self, ticket: i32, answer: Answer) {
        let Some(index) = self
            .waiting
            .iter()
            .position(|(waiting, _)| *waiting == ticket)
        else {
            // A ticket nobody here is waiting on — the clipboard's own answer,
            // most likely. Nothing to do with it, and nothing worth a line in
            // somebody's log.
            return;
        };
        let (_, waiting) = self.waiting.remove(index);

        match answer {
            Answer::Output { text } => self.copy(&text, &waiting),
            // A refusal is not a failure, and the sentence it carries is the
            // one the permission dialog used — so this can say what to go and
            // allow rather than that something went wrong.
            Answer::Refused(sentence) => sys::log(
                Level::Info,
                &format!("not allowed to: {sentence}. Allow it on the Plugins page."),
            ),
            Answer::Failed(why) => sys::log(Level::Warn, &why),
            other => sys::log(
                Level::Warn,
                &format!("that answer was not an output: {other:?}"),
            ),
        }
    }

    /// Turns a block and its output into text, and asks for it to be put on
    /// the clipboard.
    fn copy(&mut self, output: &str, waiting: &Waiting) {
        let text = format::markdown(waiting.block.as_ref(), output, waiting.shape);
        if text.is_empty() {
            // A block with neither a command nor any output is what a shell
            // with no integration leaves behind, and copying an empty fence
            // would quietly replace whatever somebody had.
            sys::log(
                Level::Info,
                "that block has no command and printed nothing, so there is nothing to copy",
            );
            return;
        }

        // Raised out of the answer to a press, which the host counts as the
        // same press: a chain is one gesture.
        if let Some(ticket) = sys::ask(&Request::Copy { text }) {
            // Remembered only so that its answer is not mistaken for an
            // output. Nothing is done with it.
            self.waiting.retain(|(waiting, _)| *waiting != ticket);
        }
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;

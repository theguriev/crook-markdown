//! What a block reads as, once it is text somebody is going to paste.
//!
//! Two shapes, and the difference between them is where it is going. A fenced
//! block goes in a message: it is the command and what it printed, and nothing
//! anybody has to read past. A report goes in an issue: it says what was run,
//! how it ended, where, and on which branch — the four things somebody reading
//! a bug always asks for and nobody ever pastes.
//!
//! Nothing here talks to the host. That is deliberate and it is most of why
//! this file exists: the formatting is the part with rules in it, so it is a
//! plain module `cargo test` runs on an ordinary machine.

use crook_plugin_api::Answer;

/// The language a console fence is tagged with.
///
/// `console` rather than `sh` or `bash`, because what is in it is not a script:
/// it is a prompt, a command and its output, which is what every renderer that
/// knows the tag highlights it as.
const FENCE_LANGUAGE: &str = "console";

/// The block a plugin was run on, as much of it as it was told.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Block {
    /// The command line, as it was submitted or echoed.
    pub command: Option<String>,
    /// What it printed, without the prompt or the line it was typed on.
    pub output: Option<String>,
    /// The status the shell reported.
    pub exit: Option<i32>,
    /// Where the shell was when the command started.
    pub directory: Option<String>,
    /// The branch that directory was on.
    pub branch: Option<String>,
}

impl Block {
    /// The block an [`Answer::Block`] describes, or `None` for any other
    /// answer.
    pub fn of(answer: &Answer) -> Option<Self> {
        match answer {
            Answer::Block {
                command,
                output,
                exit,
                directory,
                branch,
            } => Some(Self {
                command: command.clone(),
                output: output.clone(),
                exit: *exit,
                directory: directory.clone(),
                branch: branch.clone(),
            }),
            _ => None,
        }
    }

    /// Whether there is anything here worth putting on a clipboard.
    ///
    /// A block with neither a command nor any output is what a shell with no
    /// integration leaves behind, and copying an empty fence would be a menu
    /// entry that silently replaces whatever somebody had copied before.
    pub fn is_empty(&self) -> bool {
        self.command
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
            && self.output.as_deref().unwrap_or_default().trim().is_empty()
    }
}

/// Which of the two shapes an entry asks for.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Shape {
    /// The command and its output in one fence.
    Fenced,
    /// The same, with a heading and the facts around it.
    Report,
}

/// One block as Markdown.
pub fn markdown(block: &Block, shape: Shape) -> String {
    match shape {
        Shape::Fenced => fenced(block),
        Shape::Report => report(block),
    }
}

/// The command and its output, in one console fence.
fn fenced(block: &Block) -> String {
    let fence = fence_for(block);
    let mut text = String::new();
    text.push_str(&fence);
    text.push_str(FENCE_LANGUAGE);
    text.push('\n');
    if let Some(command) = trimmed(&block.command) {
        // With the prompt, because a `console` fence without one is a fence
        // where the command and the first line of output look the same.
        text.push_str("$ ");
        text.push_str(command);
        text.push('\n');
    }
    if let Some(output) = trimmed(&block.output) {
        text.push_str(output);
        text.push('\n');
    }
    text.push_str(&fence);
    text.push('\n');
    text
}

/// The same, with what somebody reading a bug report asks for around it.
fn report(block: &Block) -> String {
    let mut text = String::new();

    if let Some(command) = trimmed(&block.command) {
        text.push_str("### `");
        text.push_str(command);
        text.push_str("`\n\n");
    }

    // One sentence, made of whatever was known. A shell that reported none of
    // it contributes no sentence rather than a line of "unknown, unknown".
    let mut said = String::new();
    match block.exit {
        Some(0) => said.push_str("Succeeded"),
        Some(status) => {
            said.push_str("Exited ");
            said.push_str(&status.to_string());
        }
        None => {}
    }
    if let Some(directory) = trimmed(&block.directory) {
        said.push_str(if said.is_empty() { "Ran in `" } else { " in `" });
        said.push_str(directory);
        said.push('`');
    }
    if let Some(branch) = trimmed(&block.branch) {
        said.push_str(if said.is_empty() { "On `" } else { " on `" });
        said.push_str(branch);
        said.push('`');
    }
    if !said.is_empty() {
        text.push_str(&said);
        text.push_str(".\n\n");
    }

    text.push_str(&fenced(block));
    text
}

/// The fence to use, which is longer than the longest run of backticks in what
/// it has to hold.
///
/// Three is the ordinary answer. Anything else is output that itself contains
/// a fence — a command that printed a README, a `git show` of one — and a
/// three-backtick fence around it ends in the middle of somebody's document.
fn fence_for(block: &Block) -> String {
    let longest = [block.command.as_deref(), block.output.as_deref()]
        .into_iter()
        .flatten()
        .map(longest_backtick_run)
        .max()
        .unwrap_or(0);
    "`".repeat(longest.max(2) + 1)
}

/// The longest run of backticks in `text`.
fn longest_backtick_run(text: &str) -> usize {
    let mut longest = 0;
    let mut run = 0;
    for character in text.chars() {
        if character == '`' {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }
    longest
}

/// A field with something in it, trimmed of the blank rows a terminal leaves.
fn trimmed(field: &Option<String>) -> Option<&str> {
    let text = field.as_deref()?.trim_end();
    (!text.trim().is_empty()).then_some(text)
}

#[cfg(test)]
#[path = "format_tests.rs"]
mod tests;

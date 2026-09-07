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

use crook_plugin_api::BlockFacts;

/// The language a console fence is tagged with.
///
/// `console` rather than `sh` or `bash`, because what is in it is not a script:
/// it is a prompt, a command and its output, which is what every renderer that
/// knows the tag highlights it as.
const FENCE_LANGUAGE: &str = "console";

/// Which of the two shapes an entry asks for.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Shape {
    /// The command and its output in one fence.
    Fenced,
    /// The same, with a heading and the facts around it.
    Report,
}

/// One command and what it printed, as Markdown.
///
/// `block` is what the plugin was told about the command — `None` when it was
/// told nothing, which is a plugin nobody has granted a sight of one yet — and
/// `output` is what came back from the one thing it has to ask for.
pub fn markdown(block: Option<&BlockFacts>, output: &str, shape: Shape) -> String {
    if command(block).is_none() && output.trim().is_empty() {
        // Nothing to copy: a shell with no integration leaves a block with
        // neither a command nor any output, and an empty fence on somebody's
        // clipboard is worse than nothing happening.
        return String::new();
    }
    match shape {
        Shape::Fenced => fenced(block, output),
        Shape::Report => report(block, output),
    }
}

/// The command line, when the plugin was told one.
fn command(block: Option<&BlockFacts>) -> Option<&str> {
    let ran = block?.ran.as_ref()?;
    let command = ran.command.as_deref()?.trim();
    (!command.is_empty()).then_some(command)
}

/// The command and its output, in one console fence.
fn fenced(block: Option<&BlockFacts>, output: &str) -> String {
    let command = command(block);
    let fence = fence_for(command, output);
    let mut text = String::new();
    text.push_str(&fence);
    text.push_str(FENCE_LANGUAGE);
    text.push('\n');
    if let Some(command) = command {
        // With the prompt, because a `console` fence without one is a fence
        // where the command and the first line of output look the same.
        text.push_str("$ ");
        text.push_str(command);
        text.push('\n');
    }
    let output = output.trim_end();
    if !output.trim().is_empty() {
        text.push_str(output);
        text.push('\n');
    }
    text.push_str(&fence);
    text.push('\n');
    text
}

/// The same, with what somebody reading a bug report asks for around it.
fn report(block: Option<&BlockFacts>, output: &str) -> String {
    let mut text = String::new();

    if let Some(command) = command(block) {
        text.push_str("### `");
        text.push_str(command);
        text.push_str("`\n\n");
    }

    // One sentence, made of whatever was known. A shell that reported none of
    // it — or a plugin that was granted none of it — contributes no sentence
    // rather than a line of "unknown, unknown".
    let mut said = String::new();
    match block
        .and_then(|block| block.ran.as_ref())
        .and_then(|ran| ran.exit)
    {
        Some(0) => said.push_str("Succeeded"),
        Some(status) => {
            said.push_str("Exited ");
            said.push_str(&status.to_string());
        }
        None => {}
    }
    if let Some(place) = block.and_then(|block| block.place.as_ref()) {
        said.push_str(if said.is_empty() { "Ran in `" } else { " in `" });
        said.push_str(&place.directory);
        said.push('`');
        if let Some(branch) = place.branch.as_deref() {
            said.push_str(" on `");
            said.push_str(branch);
            said.push('`');
        }
    }
    if !said.is_empty() {
        text.push_str(&said);
        text.push_str(".\n\n");
    }

    text.push_str(&fenced(block, output));
    text
}

/// The fence to use, which is longer than the longest run of backticks in what
/// it has to hold.
///
/// Three is the ordinary answer. Anything else is output that itself contains
/// a fence — a command that printed a README, a `git show` of one — and a
/// three-backtick fence around it ends in the middle of somebody's document.
fn fence_for(command: Option<&str>, output: &str) -> String {
    let longest = [command.unwrap_or_default(), output]
        .into_iter()
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

#[cfg(test)]
#[path = "format_tests.rs"]
mod tests;

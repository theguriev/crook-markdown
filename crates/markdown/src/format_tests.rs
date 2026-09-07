//! What comes out, for the cases the shape of the text depends on.

use super::*;
use crook_plugin_api::{Place, Ran};

/// A block as the host describes one to a plugin granted everything it asks.
fn block() -> BlockFacts {
    BlockFacts {
        key: 7,
        ran: Some(Ran {
            command: Some(String::from("cargo test")),
            exit: Some(0),
        }),
        place: Some(Place {
            directory: String::from("/home/eugen/Work/crook"),
            branch: Some(String::from("blocks-helpers")),
            worktree: false,
        }),
    }
}

const OUTPUT: &str = "running 3 tests\nok";

#[test]
fn a_fence_holds_the_command_with_a_prompt_and_the_output_under_it() {
    // A `console` fence with no prompt in it is a fence where the command and
    // the first line of output look like the same thing.
    assert_eq!(
        markdown(Some(&block()), OUTPUT, Shape::Fenced),
        "```console\n$ cargo test\nrunning 3 tests\nok\n```\n"
    );
}

#[test]
fn a_report_says_what_became_of_it_and_where() {
    assert_eq!(
        markdown(Some(&block()), OUTPUT, Shape::Report),
        "### `cargo test`\n\nSucceeded in `/home/eugen/Work/crook` on `blocks-helpers`.\n\n\
         ```console\n$ cargo test\nrunning 3 tests\nok\n```\n"
    );
}

#[test]
fn a_command_that_failed_says_the_status_rather_than_that_it_succeeded() {
    let failed = BlockFacts {
        ran: Some(Ran {
            command: Some(String::from("cargo test")),
            exit: Some(101),
        }),
        ..block()
    };

    assert!(
        markdown(Some(&failed), OUTPUT, Shape::Report)
            .contains("Exited 101 in `/home/eugen/Work/crook`")
    );
}

#[test]
fn what_the_shell_never_said_contributes_nothing_rather_than_a_word_for_it() {
    // The ordinary state on the far side of an `ssh`: a command and its output
    // and nothing else. A report that filled the gaps with "unknown" would be
    // a report nobody can paste.
    let bare = BlockFacts {
        key: 3,
        ran: Some(Ran {
            command: Some(String::from("uname -a")),
            exit: None,
        }),
        place: None,
    };

    assert_eq!(
        markdown(Some(&bare), "Linux", Shape::Report),
        "### `uname -a`\n\n```console\n$ uname -a\nLinux\n```\n"
    );
}

#[test]
fn a_plugin_that_was_told_nothing_still_copies_what_it_was_handed() {
    // Granted no sight of the command, so there is no command and no status —
    // and the output is the one thing it did ask for and was allowed. What
    // comes out is a fence of that, which is worth more than nothing.
    assert_eq!(
        markdown(None, "Linux", Shape::Fenced),
        "```console\nLinux\n```\n"
    );
}

#[test]
fn a_command_that_printed_nothing_is_a_fence_with_one_line_in_it() {
    assert_eq!(
        markdown(Some(&block()), "   \n", Shape::Fenced),
        "```console\n$ cargo test\n```\n"
    );
}

#[test]
fn output_that_holds_a_fence_of_its_own_is_wrapped_in_a_longer_one() {
    // `cat README.md`, `git show` of one, anything that prints a code block.
    // Three backticks around three backticks ends the fence in the middle of
    // somebody's document, and what is pasted then is half a file.
    let readme = BlockFacts {
        key: 3,
        ran: Some(Ran {
            command: Some(String::from("cat README.md")),
            exit: Some(0),
        }),
        place: None,
    };

    let text = markdown(
        Some(&readme),
        "Run it:\n\n```sh\ncargo run\n```",
        Shape::Fenced,
    );

    assert!(text.starts_with("````console\n"), "{text}");
    assert!(text.ends_with("````\n"), "{text}");
    assert!(text.contains("```sh\ncargo run\n```"), "{text}");
}

#[test]
fn a_block_with_neither_a_command_nor_any_output_is_nothing_to_copy() {
    // What a shell with no integration leaves behind. Copying an empty fence
    // would be an entry that quietly replaces whatever was on the clipboard.
    assert!(markdown(None, "  \n ", Shape::Fenced).is_empty());
    assert!(
        markdown(
            Some(&BlockFacts {
                key: 1,
                ran: Some(Ran::default()),
                place: None
            }),
            "",
            Shape::Report
        )
        .is_empty()
    );
    assert!(!markdown(Some(&block()), OUTPUT, Shape::Fenced).is_empty());
}

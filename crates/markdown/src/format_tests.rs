//! What comes out, for the cases the shape of the text depends on.

use super::*;

fn block() -> Block {
    Block {
        command: Some(String::from("cargo test")),
        output: Some(String::from("running 3 tests\nok")),
        exit: Some(0),
        directory: Some(String::from("/home/eugen/Work/crook")),
        branch: Some(String::from("blocks-helpers")),
    }
}

#[test]
fn a_fence_holds_the_command_with_a_prompt_and_the_output_under_it() {
    // A `console` fence with no prompt in it is a fence where the command and
    // the first line of output look like the same thing.
    assert_eq!(
        markdown(&block(), Shape::Fenced),
        "```console\n$ cargo test\nrunning 3 tests\nok\n```\n"
    );
}

#[test]
fn a_report_says_what_became_of_it_and_where() {
    assert_eq!(
        markdown(&block(), Shape::Report),
        "### `cargo test`\n\nSucceeded in `/home/eugen/Work/crook` on `blocks-helpers`.\n\n\
         ```console\n$ cargo test\nrunning 3 tests\nok\n```\n"
    );
}

#[test]
fn a_command_that_failed_says_the_status_rather_than_that_it_succeeded() {
    let failed = Block {
        exit: Some(101),
        ..block()
    };

    assert!(markdown(&failed, Shape::Report).contains("Exited 101 in `/home/eugen/Work/crook`"));
}

#[test]
fn what_the_shell_never_said_contributes_nothing_rather_than_a_word_for_it() {
    // The ordinary state on the far side of an `ssh`: a command and its output
    // and nothing else. A report that filled the gaps with "unknown" would be
    // a report nobody can paste.
    let bare = Block {
        command: Some(String::from("uname -a")),
        output: Some(String::from("Linux")),
        ..Block::default()
    };

    assert_eq!(
        markdown(&bare, Shape::Report),
        "### `uname -a`\n\n```console\n$ uname -a\nLinux\n```\n"
    );
}

#[test]
fn a_command_that_printed_nothing_is_a_fence_with_one_line_in_it() {
    let quiet = Block {
        output: Some(String::from("   \n")),
        ..block()
    };

    assert_eq!(
        markdown(&quiet, Shape::Fenced),
        "```console\n$ cargo test\n```\n"
    );
}

#[test]
fn output_that_holds_a_fence_of_its_own_is_wrapped_in_a_longer_one() {
    // `cat README.md`, `git show` of one, anything that prints a code block.
    // Three backticks around three backticks ends the fence in the middle of
    // somebody's document, and what is pasted then is half a file.
    let readme = Block {
        command: Some(String::from("cat README.md")),
        output: Some(String::from("Run it:\n\n```sh\ncargo run\n```")),
        ..Block::default()
    };

    let text = markdown(&readme, Shape::Fenced);

    assert!(text.starts_with("````console\n"), "{text}");
    assert!(text.ends_with("````\n"), "{text}");
    assert!(text.contains("```sh\ncargo run\n```"), "{text}");
}

#[test]
fn a_block_with_neither_a_command_nor_any_output_is_nothing_to_copy() {
    // What a shell with no integration leaves behind. Copying an empty fence
    // would be an entry that quietly replaces whatever was on the clipboard.
    assert!(Block::default().is_empty());
    assert!(
        Block {
            output: Some(String::from("  \n ")),
            ..Block::default()
        }
        .is_empty()
    );
    assert!(!block().is_empty());
}

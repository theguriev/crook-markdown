//! The loop, with the host stubbed out: an entry is pressed, the output is
//! asked for, and what comes back is asked to be put on the clipboard.

use super::*;
use crate::sys::stub;
use crook_plugin_api::{BlockFacts, Node, Place, Ran};

/// A render of this plugin's group, about one command.
fn render(command: &str) -> Render {
    Render {
        slot: String::from(SLOT),
        entry: String::from("markdown"),
        subject: Some(Subject::Block(BlockFacts {
            key: 7,
            ran: Some(Ran {
                command: Some(command.to_owned()),
                exit: Some(0),
            }),
            place: Some(Place {
                directory: String::from("/home/eugen/Work/crook"),
                branch: Some(String::from("main")),
                worktree: false,
            }),
        })),
    }
}

/// A plugin that has built and been drawn once, with everything the build
/// asked for forgotten.
fn drawn(command: &str) -> Markdown {
    stub::forget();
    let mut plugin = Markdown::new();
    plugin.build();
    plugin.render(&render(command));
    let _ = stub::taken();
    plugin
}

/// What a request came out as, in order.
fn asked() -> Vec<Request> {
    stub::taken()
        .requests
        .into_iter()
        .map(|(_, request)| request)
        .collect()
}

#[test]
fn building_puts_one_group_in_the_menu_and_offers_two_actions() {
    stub::forget();
    let mut plugin = Markdown::new();
    plugin.build();

    let asked = stub::taken();
    assert_eq!(
        asked.contributions,
        vec![(String::from(SLOT), String::from("markdown"), ORDER)]
    );
    assert_eq!(
        asked.actions,
        vec![
            (String::from(FENCED), String::new()),
            (String::from(REPORT), String::new()),
        ],
        "neither is offered to a palette: an entry about a block is about nothing there"
    );
}

#[test]
fn a_render_about_no_block_at_all_still_draws_the_group() {
    // What a plugin granted nothing sees, and what one drawn before a subject
    // exists sees. Both draw: being refused says what to allow, and a menu
    // that quietly lost its rows says nothing.
    stub::forget();
    let mut plugin = Markdown::new();
    plugin.build();

    let drawn = plugin.render(&Render {
        slot: String::from(SLOT),
        entry: String::from("markdown"),
        subject: None,
    });

    assert_ne!(drawn, Node::Empty);
}

#[test]
fn pressing_an_entry_asks_what_it_printed_and_then_asks_for_the_clipboard() {
    let mut plugin = drawn("cargo test");

    plugin.run(FENCED);
    let (ticket, request) = stub::last_ticket().expect("a ticket was handed out");
    assert_eq!(request, Request::Output, "it asked for something else");
    let _ = stub::taken();

    plugin.deliver(
        ticket,
        Answer::Output {
            text: String::from("ok"),
        },
    );
    assert_eq!(
        asked(),
        vec![Request::Copy {
            text: String::from("```console\n$ cargo test\nok\n```\n"),
        }],
        "the command it was drawn for is the command it copied"
    );
}

#[test]
fn two_entries_pressed_in_a_row_are_answered_as_the_shapes_they_asked_for() {
    // The reason a ticket is remembered at all: an answer says nothing about
    // what it was for, and a plugin that kept one slot would paste a bare
    // fence for the entry that asked for a report.
    let mut plugin = drawn("cargo test");

    plugin.run(FENCED);
    let (fenced_ticket, _) = stub::last_ticket().expect("a ticket");
    plugin.run(REPORT);
    let (report_ticket, _) = stub::last_ticket().expect("a ticket");
    let _ = stub::taken();

    plugin.deliver(
        report_ticket,
        Answer::Output {
            text: String::from("ok"),
        },
    );
    plugin.deliver(
        fenced_ticket,
        Answer::Output {
            text: String::from("ok"),
        },
    );

    let copied: Vec<String> = stub::taken()
        .requests
        .into_iter()
        .filter_map(|(_, request)| match request {
            Request::Copy { text } => Some(text),
            _ => None,
        })
        .collect();
    assert!(copied[0].starts_with("### `cargo test`"), "{copied:?}");
    assert!(copied[1].starts_with("```console"), "{copied:?}");
}

#[test]
fn a_refusal_says_what_to_allow_rather_than_that_something_went_wrong() {
    // The state every plugin installs in. What comes back is the sentence the
    // permission dialog used, verbatim, which is the only thing that makes
    // this line worth printing.
    let mut plugin = drawn("cargo test");

    plugin.run(FENCED);
    let (ticket, _) = stub::last_ticket().expect("a ticket");
    let _ = stub::taken();
    plugin.deliver(
        ticket,
        Answer::Refused(String::from(
            "Read the command you run it on, and what it printed",
        )),
    );

    let asked = stub::taken();
    assert!(asked.requests.is_empty(), "it asked for something anyway");
    assert!(
        asked.logs[0]
            .1
            .contains("Read the command you run it on, and what it printed"),
        "{:?}",
        asked.logs
    );
    assert!(asked.logs[0].1.contains("Plugins page"), "{:?}", asked.logs);
}

#[test]
fn a_block_with_nothing_in_it_replaces_nobody_s_clipboard() {
    stub::forget();
    let mut plugin = Markdown::new();
    plugin.build();
    // Drawn for a block nothing is known about, and it printed nothing either.
    plugin.render(&Render {
        slot: String::from(SLOT),
        entry: String::from("markdown"),
        subject: None,
    });
    let _ = stub::taken();

    plugin.run(FENCED);
    let (ticket, _) = stub::last_ticket().expect("a ticket");
    let _ = stub::taken();
    plugin.deliver(
        ticket,
        Answer::Output {
            text: String::from("   "),
        },
    );

    let asked = stub::taken();
    assert!(
        asked.requests.is_empty(),
        "an empty fence went on the clipboard"
    );
    assert!(!asked.logs.is_empty(), "and nothing said why");
}

#[test]
fn an_answer_to_a_ticket_nobody_is_waiting_on_is_ignored() {
    let mut plugin = drawn("cargo test");

    plugin.deliver(
        404,
        Answer::Output {
            text: String::from("ok"),
        },
    );

    assert!(stub::taken().requests.is_empty());
}

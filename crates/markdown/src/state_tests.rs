//! The loop, with the host stubbed out: an entry is pressed, a block is asked
//! for, and what comes back is asked to be put on the clipboard.

use super::*;
use crate::sys::stub;

/// A block as the host describes one.
fn answer() -> Answer {
    Answer::Block {
        command: Some(String::from("cargo test")),
        output: Some(String::from("ok")),
        exit: Some(0),
        directory: Some(String::from("/home/eugen/Work/crook")),
        branch: Some(String::from("main")),
    }
}

/// A plugin that has built, with everything the build asked for forgotten.
fn built() -> Markdown {
    stub::forget();
    let mut plugin = Markdown::new();
    plugin.build();
    plugin
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
fn pressing_an_entry_asks_for_the_block_and_answering_asks_for_the_clipboard() {
    let mut plugin = built();
    let _ = stub::taken();

    plugin.run(FENCED);
    let asked = stub::taken();
    assert_eq!(asked.requests.len(), 1, "{asked:?}");
    let (ticket, request) = asked.requests[0].clone();
    assert_eq!(request, Request::Block);

    plugin.deliver(ticket, answer());
    let asked = stub::taken();
    assert_eq!(
        asked
            .requests
            .iter()
            .map(|(_, request)| request.clone())
            .collect::<Vec<_>>(),
        vec![Request::Copy {
            text: String::from("```console\n$ cargo test\nok\n```\n")
        }]
    );
}

#[test]
fn two_entries_pressed_in_a_row_are_answered_as_the_shapes_they_asked_for() {
    // The reason a ticket is remembered at all: an answer says nothing about
    // what it was for, and a plugin that kept one slot would paste a bare
    // fence for the entry that asked for a report.
    let mut plugin = built();
    let _ = stub::taken();

    plugin.run(FENCED);
    plugin.run(REPORT);
    let asked = stub::taken();
    let (fenced_ticket, _) = asked.requests[0].clone();
    let (report_ticket, _) = asked.requests[1].clone();

    plugin.deliver(report_ticket, answer());
    plugin.deliver(fenced_ticket, answer());

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
    let mut plugin = built();
    let _ = stub::taken();

    plugin.run(FENCED);
    let (ticket, _) = stub::taken().requests[0].clone();
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
    let mut plugin = built();
    let _ = stub::taken();

    plugin.run(FENCED);
    let (ticket, _) = stub::taken().requests[0].clone();
    plugin.deliver(
        ticket,
        Answer::Block {
            command: None,
            output: Some(String::from("   ")),
            exit: None,
            directory: None,
            branch: None,
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
    let mut plugin = built();
    let _ = stub::taken();

    plugin.deliver(404, answer());

    assert!(stub::taken().requests.is_empty());
}

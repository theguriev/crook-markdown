//! What is offered, for the blocks where one of the two entries would say
//! nothing.

use super::*;
use crook_plugin_api::{Place, Ran};

fn items(subject: Option<&Subject>) -> Vec<String> {
    match menu(subject) {
        Node::Menu { items, .. } => items.into_iter().map(|item| item.label).collect(),
        other => panic!("a menu group is a menu: {other:?}"),
    }
}

fn block(ran: Option<Ran>, place: Option<Place>) -> Subject {
    Subject::Block(BlockFacts { key: 7, ran, place })
}

#[test]
fn a_command_with_a_status_or_a_place_is_worth_a_report() {
    let ended = block(
        Some(Ran {
            command: Some(String::from("cargo test")),
            exit: Some(101),
        }),
        None,
    );

    assert_eq!(
        items(Some(&ended)),
        vec![
            String::from("Copy as Markdown"),
            String::from("Copy as a report")
        ]
    );
}

#[test]
fn a_command_that_reported_nothing_is_offered_the_fence_alone() {
    // The far side of an `ssh`: a command and its output, and nothing else. A
    // report of that is a heading with nothing under it.
    let bare = block(
        Some(Ran {
            command: Some(String::from("uname -a")),
            exit: None,
        }),
        None,
    );

    assert_eq!(items(Some(&bare)), vec![String::from("Copy as Markdown")]);
}

#[test]
fn a_plugin_that_was_told_nothing_still_offers_both() {
    // Granted nothing, so `ran` and `place` are absent for a reason that is
    // not about the block. Both entries are drawn: the refusal a press earns
    // is a sentence somebody can act on, and a row that quietly vanished is
    // not.
    assert_eq!(
        items(Some(&block(None, None))).len(),
        2,
        "a plugin nobody has answered for lost half its menu"
    );
    assert_eq!(items(None).len(), 2, "and so did one drawn for nothing");
}

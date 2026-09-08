// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

//! The forms, read here; the frames they turn into are crate::command’s.

use channels::{ChosenSummary, DialectKind, EndpointSummary, EndpointsAnswer, ModelTag};

use super::forms::{
    AttachForm, AttachReadiness, SelectForm, SelectReadiness, models_of, ready, select_ready,
    url_is_safe,
};
use super::tables::enrolment_note;
use super::tables::{can_dispatch, endpoint_rows, tag_rows};
use crate::lang::Msg;
use crate::socket::Enrolment;
use channels::WireCommand;

fn answer() -> EndpointsAnswer {
    EndpointsAnswer {
        endpoints: vec![EndpointSummary {
            name: "house".to_owned(),
            base_url: "https://api.example.test/v1".to_owned(),
            dialect: DialectKind::OpenAi,
            models: vec!["m-large".to_owned()],
            local: false,
            has_credential: true,
        }],
        chosen: vec![ChosenSummary {
            tag: ModelTag::Main,
            endpoint: "house".to_owned(),
            model: "m-large".to_owned(),
            max_output_tokens: 8_192,
        }],
    }
}

#[test]
fn a_model_the_endpoint_never_listed_cannot_be_chosen_on_the_page_either() {
    let served = answer();
    let mut form = SelectForm::default();
    assert_eq!(select_ready(&form, &served), SelectReadiness::NeedsEndpoint);
    form.endpoint = "house".to_owned();
    assert_eq!(select_ready(&form, &served), SelectReadiness::NeedsModel);
    form.model = "m-invented".to_owned();
    assert_eq!(select_ready(&form, &served), SelectReadiness::NeedsTag);
    form.tag = Some(ModelTag::Main);
    assert_eq!(
        select_ready(&form, &served),
        SelectReadiness::ModelNotServed,
        "the refusal the server would give, given before the person presses anything"
    );
    assert!(crate::command::select_command(&form, &served).is_none());
    form.model = "m-large".to_owned();
    assert_eq!(select_ready(&form, &served), SelectReadiness::Ready);
    let command =
        crate::command::select_command(&form, &served).expect("a ready form is a command");
    match command {
        WireCommand::SelectModel {
            model,
            max_output_tokens,
            ..
        } => {
            assert_eq!(model, "m-large");
            assert_eq!(
                max_output_tokens, 0,
                "the ceiling is the model's fact; zero asks the server for it"
            );
        }
        other => panic!("a model choice is a SelectModel, not {other:?}"),
    }
}

#[test]
fn the_model_list_holds_what_the_chosen_provider_serves_and_nothing_else() {
    let mut served = answer();
    served.endpoints.push(EndpointSummary {
        name: "neighbour".to_owned(),
        base_url: "https://other.example.test/v1".to_owned(),
        dialect: DialectKind::Anthropic,
        models: vec!["n-small".to_owned()],
        local: false,
        has_credential: true,
    });
    assert_eq!(models_of(&served, "house"), vec!["m-large".to_owned()]);
    assert_eq!(models_of(&served, "neighbour"), vec!["n-small".to_owned()]);
    assert!(
        models_of(&served, "").is_empty(),
        "with no provider chosen there is nothing to choose from, and the page says so"
    );
}

#[test]
fn a_form_says_which_field_is_missing_rather_than_only_that_it_is_incomplete() {
    let mut form = AttachForm::default();
    assert_eq!(ready(&form), AttachReadiness::NeedsName);
    form.name = "house".to_owned();
    assert_eq!(ready(&form), AttachReadiness::NeedsUrl);
    form.base_url = "https://api.example.test/v1".to_owned();
    assert_eq!(ready(&form), AttachReadiness::NeedsDialect);
    form.dialect = Some(DialectKind::OpenAi);
    assert_eq!(ready(&form), AttachReadiness::Ready);
}

#[test]
fn a_credential_may_not_cross_a_plaintext_link_off_this_machine() {
    assert!(url_is_safe("https://api.example.test/v1"));
    assert!(url_is_safe("http://127.0.0.1:11434/v1"));
    assert!(url_is_safe("http://localhost:1234/v1"));
    assert!(!url_is_safe("http://api.example.test/v1"));
    assert!(!url_is_safe("ftp://api.example.test"));
    assert!(!url_is_safe("api.example.test"));

    let form = AttachForm {
        name: "house".to_owned(),
        base_url: "http://api.example.test/v1".to_owned(),
        dialect: Some(DialectKind::OpenAi),
        secret: Some("secret:house/key".to_owned()),
        admit: Vec::new(),
        auth_header: String::new(),
        declared: String::new(),
    };
    assert_eq!(ready(&form), AttachReadiness::UrlNotSafe);
    assert!(
        crate::lang::phrase(ready(&form).sentence())
            .en
            .contains("https")
    );
}

#[test]
fn a_row_says_where_it_reaches_and_whether_it_has_a_credential() {
    let rows = endpoint_rows(&answer());
    assert_eq!(rows[0].reach, Msg::SettingsOffThisMachine);
    assert_eq!(rows[0].credential, Msg::SettingsWithCredential);
    assert_eq!(rows[0].models, vec!["m-large".to_owned()]);
}

#[test]
fn every_tag_is_listed_even_when_nothing_answers_for_it() {
    let rows = tag_rows(&answer());
    assert_eq!(rows.len(), ModelTag::ALL.len());
    let main = rows.iter().find(|row| row.tag == ModelTag::Main).unwrap();
    assert_eq!(main.chosen.as_ref().unwrap().model, "m-large");
    let digest = rows.iter().find(|row| row.tag == ModelTag::Digest).unwrap();
    assert!(digest.chosen.is_none());
    assert!(
        crate::lang::phrase(digest.consequence)
            .en
            .contains("main model"),
        "an unset tag states what it costs, not that it is unset"
    );
}

#[test]
fn an_unready_form_yields_no_command_at_all() {
    let mut form = AttachForm {
        name: "house".to_owned(),
        base_url: "http://api.example.test/v1".to_owned(),
        dialect: Some(DialectKind::OpenAi),
        secret: None,
        admit: Vec::new(),
        auth_header: String::new(),
        declared: String::new(),
    };
    assert!(
        crate::command::attach_command(&form).is_none(),
        "a half-built command would be a second statement of what a complete form is"
    );
    form.base_url = "https://api.example.test/v1".to_owned();
    let Some(WireCommand::AttachEndpoint {
        name,
        base_url,
        dialect,
        secret,
        idem,
        ..
    }) = crate::command::attach_command(&form)
    else {
        panic!("a ready form asks to attach");
    };
    assert_eq!(name.as_str(), "house");
    assert_eq!(base_url, "https://api.example.test/v1");
    assert_eq!(dialect, DialectKind::OpenAi);
    assert_eq!(secret, None);
    // Pressing twice attaches once: the key is derived from what was
    // entered, not from when the button was pressed.
    let Some(WireCommand::AttachEndpoint { idem: again, .. }) =
        crate::command::attach_command(&form)
    else {
        panic!("a ready form asks to attach");
    };
    assert_eq!(idem, again);
}

#[test]
fn an_enrolment_leaves_a_reference_and_a_sentence_that_says_where_the_key_went() {
    let (reference, said) = enrolment_note(
        crate::lang::Lang::En,
        &Enrolment::Stored {
            reference: "secret:house/key".to_owned(),
        },
    );
    assert_eq!(reference.as_deref(), Some("secret:house/key"));
    assert!(said.contains("only in the vault"));

    // A refusal leaves no reference: a form that kept one would ask
    // the server to redeem a credential nobody stored.
    let (reference, said) = enrolment_note(
        crate::lang::Lang::En,
        &Enrolment::Refused {
            reason: "203.0.113.7 is not on this machine".to_owned(),
        },
    );
    assert_eq!(reference, None);
    assert!(said.contains("not on this machine"));
}

#[test]
fn the_page_answers_whether_this_city_can_be_dispatched_to() {
    assert!(can_dispatch(&answer()));
    let empty = EndpointsAnswer {
        endpoints: Vec::new(),
        chosen: Vec::new(),
    };
    assert!(!can_dispatch(&empty));
    let rows = tag_rows(&empty);
    assert!(
        rows.iter()
            .find(|row| row.tag == ModelTag::Main)
            .is_some_and(|row| crate::lang::phrase(row.consequence).en.contains("refused")),
        "the page states the consequence a person is about to hit"
    );
}

#[test]
fn declared_models_join_the_ticked_ones_once_each_in_the_order_written() {
    let form = AttachForm {
        admit: vec!["m-large".to_owned(), "m-small".to_owned()],
        declared: " m-small, m-vision \n\n m-large \nm-vision,,m-code ".to_owned(),
        ..AttachForm::default()
    };
    assert_eq!(
        form.admitted(),
        vec!["m-large", "m-small", "m-vision", "m-code"],
        "ticked first, then what was written; a name twice is one name; blanks are not names"
    );
    assert!(
        AttachForm::default().admitted().is_empty(),
        "nothing ticked and nothing written admits everything, which is an empty list on the wire"
    );
}

#[test]
fn an_empty_header_name_leaves_the_choice_to_the_dialect() {
    let mut form = AttachForm::default();
    assert_eq!(form.header_name(), None);
    form.auth_header = "   ".to_owned();
    assert_eq!(form.header_name(), None, "blank is empty");
    form.auth_header = " x-goog-api-key ".to_owned();
    assert_eq!(form.header_name().as_deref(), Some("x-goog-api-key"));
}

#[test]
fn a_ready_form_sends_its_header_name_and_its_declared_models_on_both_verbs() {
    let form = AttachForm {
        name: "house".to_owned(),
        base_url: "https://api.example.test/v1".to_owned(),
        dialect: Some(DialectKind::Anthropic),
        secret: Some("secret:house/key".to_owned()),
        admit: Vec::new(),
        auth_header: " x-api-key ".to_owned(),
        declared: "claude-a\nclaude-b".to_owned(),
    };
    let Some(WireCommand::AttachEndpoint {
        auth_header, admit, ..
    }) = crate::command::attach_command(&form)
    else {
        panic!("a ready form asks to attach");
    };
    assert_eq!(auth_header.as_deref(), Some("x-api-key"));
    assert_eq!(
        admit,
        vec!["claude-a", "claude-b"],
        "what was written registers when the endpoint cannot list its models"
    );
    let Some(WireCommand::ProbeEndpoint { auth_header, .. }) = crate::command::probe_command(&form)
    else {
        panic!("a ready form may ask what it serves");
    };
    assert_eq!(
        auth_header.as_deref(),
        Some("x-api-key"),
        "the probe asks with the same header the attachment will use"
    );
}

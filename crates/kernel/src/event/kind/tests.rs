// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The kind set's two invariants: every kind spells itself once, and the
//! window partition holds the kinds the criterion puts there.

use super::*;
use crate::error::{AxCode, Carrier};
use std::collections::BTreeSet;

#[test]
fn every_kind_spells_itself_once_and_exactly_nine_reach_the_window() {
    let names: BTreeSet<String> = EventKind::ALL
        .iter()
        .map(|k| serde_json::to_string(k).unwrap())
        .collect();
    assert_eq!(
        names.len(),
        EventKind::ALL.len(),
        "serde spellings must be unique"
    );
    let in_window: Vec<EventKind> = EventKind::ALL
        .into_iter()
        .filter(|k| k.window_class() == WindowClass::InWindow)
        .collect();
    assert_eq!(in_window.len(), 9);
    for k in [
        EventKind::PromptAssembled,
        EventKind::ModelCalled,
        EventKind::ModelReturned,
        EventKind::ToolCalled,
        EventKind::ToolResult,
        EventKind::ResultOffloaded,
        EventKind::SteerReceived,
        EventKind::SignalConsumed,
        EventKind::AdviserAnswered,
    ] {
        assert_eq!(k.window_class(), WindowClass::InWindow);
    }
    assert_eq!(
        serde_json::to_string(&EventKind::CityInitialized).unwrap(),
        "\"city_initialized\""
    );
}

#[test]
fn carrier_declarations_cover_all_35_codes() {
    let mut loadtime = 0;
    let mut gate = 0;
    let mut tool = 0;
    for code in AxCode::ALL {
        match code.carrier() {
            Carrier::Loadtime => loadtime += 1,
            Carrier::Event(EventKind::GateDenied) => gate += 1,
            Carrier::Event(EventKind::ToolResult) => tool += 1,
            Carrier::Event(_) => {}
        }
    }
    assert_eq!(loadtime, 5, "loadtime whitelist is closed at five");
    assert_eq!(gate, 7);
    assert_eq!(tool, 18);
    assert_eq!(
        AxCode::BudgetExhausted.carrier(),
        Carrier::Event(EventKind::BudgetLimit)
    );
    assert_eq!(
        AxCode::ApprovalPending.carrier(),
        Carrier::Event(EventKind::ApprovalRequested)
    );
    assert_eq!(
        AxCode::ApprovalDenied.carrier(),
        Carrier::Event(EventKind::ApprovalResolved)
    );
    assert_eq!(
        AxCode::Provider.carrier(),
        Carrier::Event(EventKind::ProviderDegraded)
    );
    assert_eq!(
        AxCode::EndpointDialectUnsupported.carrier(),
        Carrier::Event(EventKind::EndpointLost)
    );
    assert_eq!(
        AxCode::LoopSuspected.carrier(),
        Carrier::Event(EventKind::WatchdogFired)
    );
}

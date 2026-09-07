// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The head: the four questions a person arrives with.

use crate::lang::{Lang, Msg, fill, say};

/// One of the four things the head says, and the four are fixed.
///
/// A page that reports whatever it happens to know reports a different
/// set every time it is opened. These four were chosen because they are
/// the questions a person asks before deciding what to do next, and the
/// set does not grow when a new field appears on the wire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fact {
    /// What the fact says.
    pub said: String,
    /// Whether the city could answer it at all. A fact it cannot answer
    /// is drawn quiet and explained in the scope line, never omitted:
    /// a missing row reads as "nothing to report", which is the one
    /// thing that is not true.
    pub known: bool,
}

/// What this client can say about a session, in the fixed order the head
/// reads them.
///
/// Pure, so the assertion that the third one is never invented can be
/// written against a value rather than against markup.
#[must_use]
pub fn head_facts(lang: Lang, row: &crate::app::RunRow) -> [Fact; 4] {
    let spent = match row.spent {
        Some(amount) => Fact {
            said: fill(
                say(lang, Msg::SessionSpentIs),
                &[("amount", &crate::readout::render_usd(amount))],
            ),
            known: true,
        },
        None => Fact {
            said: fill(
                say(lang, Msg::SessionSpentIs),
                &[("amount", say(lang, Msg::SessionsUnpriced))],
            ),
            known: false,
        },
    };
    let gate = match row.gate.as_deref() {
        Some(named) => Fact {
            said: fill(say(lang, Msg::SessionAtGate), &[("gate", named)]),
            known: true,
        },
        None => Fact {
            said: say(lang, Msg::SessionNoGate).to_owned(),
            known: true,
        },
    };
    // The one the wire cannot carry. `runtime::tools::status` measures
    // it and thirteen fields beside it, and not one of them is on the
    // wire — so this city knows the number and no query returns it.
    // Drawn as a rule with the reason in the scope line: a plausible
    // figure here would be the interface inventing the single fact a
    // person is most likely to act on.
    let context = Fact {
        said: say(lang, Msg::SessionContextUnknown).to_owned(),
        known: false,
    };
    let handoff = match row.handoff_at_turn {
        None => Fact {
            said: say(lang, Msg::SessionHandoffNone).to_owned(),
            known: true,
        },
        Some(at) => {
            let ago = row.turns.saturating_sub(at);
            Fact {
                said: if ago == 0 {
                    say(lang, Msg::SessionHandoffJust).to_owned()
                } else {
                    fill(say(lang, Msg::SessionHandoffAt), &[("n", &ago.to_string())])
                },
                known: true,
            }
        }
    };
    [spent, gate, context, handoff]
}

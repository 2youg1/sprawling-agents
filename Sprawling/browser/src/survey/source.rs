// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

pub const DRAWN_IN: [&str; 3] = ["tsx", "ts", "html"];

pub const ENOUGH: usize = 16;

#[derive(Default)]
pub struct Sources {
    spelled: Vec<(String, String)>,
}

impl Sources {
    #[must_use]
    pub fn of(spelled: Vec<(String, String)>) -> Sources {
        let mut spelled = spelled;

        spelled.sort_by(|left, right| {
            right
                .0
                .len()
                .cmp(&left.0.len())
                .then_with(|| left.0.cmp(&right.0))
        });
        Sources { spelled }
    }

    #[must_use]
    pub fn locate(&self, class: &str) -> Option<&str> {
        if class.is_empty() {
            return None;
        }
        self.spelled
            .iter()
            .find(|(literal, _)| class.contains(literal.as_str()))
            .map(|(_, at)| at.as_str())
    }
}

#[must_use]
pub fn literals(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(open) = rest.find('"') {
        let Some(tail) = rest.get(open.saturating_add(1)..) else {
            break;
        };
        let Some(close) = tail.find('"') else {
            break;
        };
        if let Some(inner) = tail.get(..close)
            && !inner.is_empty()
        {
            out.push(inner.to_owned());
        }
        let Some(next) = tail.get(close.saturating_add(1)..) else {
            break;
        };
        rest = next;
    }
    out
}

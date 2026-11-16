// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The three verbs a workshop answers to, and the refusal that names
//! them.

use kernel::{AxCode, AxError};

/// The three things this tool does. Exhaustive: an unknown verb is
/// refused rather than rounded to the harmless one, because the harmless
/// one here would silently drop a graph somebody meant to run.
pub(super) enum Op {
    LayOut,
    Question,
    Judge,
}

impl Op {
    pub(super) fn parse(raw: &str) -> Result<Op, AxError> {
        match raw {
            "lay_out" => Ok(Op::LayOut),
            "question" => Ok(Op::Question),
            "judge" => Ok(Op::Judge),
            other => Err(AxError::failure(
                AxCode::InvalidArgs,
                "run a workshop",
                format!("no such operation: {other}"),
            )
            .with_recovery("lay_out to split the work, question then judge to close the join")),
        }
    }
}

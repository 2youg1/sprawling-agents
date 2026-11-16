// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError, ImageRef, ToolCall};

pub(super) const LINE_CAP: u16 = 512;

pub(super) enum Found {
    Chosen(kernel::Address),

    Text(String),

    Picture { picture: ImageRef, bytes: Vec<u8> },
}

pub(super) struct Interval {
    pub(super) offset: u64,
    pub(super) limit: u16,
}

pub(super) struct Taken {
    pub(super) text: String,
    pub(super) total_lines: Option<usize>,
    pub(super) next_offset: Option<usize>,

    pub(super) lines: usize,
    pub(super) total: usize,
}

impl Interval {
    pub(super) fn asked_for(call: &ToolCall) -> Result<Interval, AxError> {
        let args = call.args.as_map();
        let whole = |field: &'static str| -> Result<Option<u64>, AxError> {
            match args.get(field) {
                None => Ok(None),
                Some(value) => value.as_u64().map(Some).ok_or_else(|| {
                    AxError::failure(
                        AxCode::InvalidArgs,
                        "read",
                        format!("`{field}` is not a whole number of lines"),
                    )
                    .with_recovery(
                        "pass `offset` and `limit` as non-negative integers, or leave them out \
                         to read from the start",
                    )
                }),
            }
        };
        let offset = whole("offset")?.map_or(0, |line| line.saturating_sub(1));

        let limit = whole("limit")?.map_or(LINE_CAP, |asked| {
            u16::try_from(asked).map_or(LINE_CAP, |fits| fits.min(LINE_CAP))
        });
        if limit == 0 {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "read",
                "`limit` is zero, which asks for no lines at all",
            )
            .with_recovery("pass a `limit` from 1 to 512, or leave it out"));
        }
        Ok(Interval { offset, limit })
    }

    pub(super) fn cut(&self, text: &str) -> Taken {
        let mut lines = text.split_inclusive('\n');

        let start = usize::try_from(self.offset).ok();
        let mut from = 0usize;
        let mut skipped = 0usize;
        while Some(skipped) != start {
            let Some(line) = lines.next() else {
                return Taken {
                    text: String::new(),
                    total_lines: Some(skipped),
                    next_offset: None,
                    lines: 0,
                    total: skipped,
                };
            };
            from = from.saturating_add(line.len());
            skipped = skipped.saturating_add(1);
        }
        let mut lines = lines.peekable();
        if lines.peek().is_none() {
            return Taken {
                text: String::new(),
                total_lines: Some(skipped),
                next_offset: None,
                lines: 0,
                total: skipped,
            };
        }
        let budget = kernel::consts_policy::INTERVAL_CAP_BYTES;
        let mut taken = 0usize;
        let mut spent = 0usize;
        while taken < usize::from(self.limit) {
            let Some(line) = lines.peek() else { break };
            let after = spent.saturating_add(line.len());

            if after > budget && taken > 0 {
                break;
            }
            spent = after;
            taken = taken.saturating_add(1);
            lines.next();
        }
        let text = text
            .get(from..from.saturating_add(spent))
            .unwrap_or_default()
            .to_owned();
        let end = skipped.saturating_add(taken);
        match lines.count() {
            0 => Taken {
                text,
                total_lines: None,
                next_offset: None,
                lines: taken,
                total: end,
            },
            left => Taken {
                text,
                total_lines: Some(end.saturating_add(left)),
                next_offset: Some(end),
                lines: taken,
                total: end.saturating_add(left),
            },
        }
    }
}

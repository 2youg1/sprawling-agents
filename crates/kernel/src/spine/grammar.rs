// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Table grammar: locating the table and reading its rows.

use crate::locator::Locator;
use crate::node_id::NodeId;

use super::row::{
    EvidenceCell, ROADMAP_COLUMNS, ROADMAP_STATUS_SPELLINGS, RoadmapRow, RoadmapShape,
    RoadmapStatus,
};

fn parse_status(raw: &str) -> Option<RoadmapStatus> {
    ROADMAP_STATUS_SPELLINGS
        .iter()
        .find(|(_, spelling)| spelling.eq_ignore_ascii_case(raw))
        .map(|(status, _)| *status)
}

pub(crate) fn split_table_row(line: &str) -> Option<Vec<String>> {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') {
        return None;
    }
    let inner = trimmed.strip_prefix('|')?.strip_suffix('|')?;
    Some(
        inner
            .split('|')
            .map(|cell| cell.trim().to_owned())
            .collect(),
    )
}

pub(crate) fn is_separator(cells: &[String]) -> bool {
    !cells.is_empty()
        && cells
            .iter()
            .all(|c| !c.is_empty() && c.chars().all(|ch| ch == '-' || ch == ':'))
}

/// Whether a line is a body row of the roadmap: a table row that is
/// neither the header nor the separator. The one test, shared by the
/// parser and both writers, so "which lines are the plan" has a single
/// answer.
pub(crate) fn body_row(cells: &[String]) -> bool {
    cells.len() == ROADMAP_COLUMNS && !is_separator(cells)
}

/// The lines of the first table, as an inclusive range over
/// `text.split('\n')`. `None` when the text carries no table.
pub(crate) fn locate_table(text: &str) -> Option<(usize, usize)> {
    let mut first = None;
    let mut last = 0;
    for (n, line) in text.split('\n').enumerate() {
        if split_table_row(line).is_some() {
            if first.is_none() {
                first = Some(n);
            }
            last = n;
        } else if first.is_some() {
            break;
        }
    }
    first.map(|start| (start, last))
}

/// Finds the first pipe table and checks the six-column contract. Pure
/// string work, no I/O; `crate::plan` builds the tree from what comes
/// back.
#[must_use]
pub fn check_roadmap_shape(text: &str) -> RoadmapShape {
    let mut rows = Vec::new();
    let mut problems = Vec::new();
    let mut in_table = false;
    let mut header_seen = false;
    for (n, line) in text.lines().enumerate() {
        let line_no = n.saturating_add(1);
        let Some(cells) = split_table_row(line) else {
            if in_table {
                break; // first table ended
            }
            continue;
        };
        in_table = true;
        if cells.len() != ROADMAP_COLUMNS {
            problems.push(format!(
                "line {line_no}: {} columns, the roadmap table has exactly {ROADMAP_COLUMNS}",
                cells.len()
            ));
            continue;
        }
        if !header_seen {
            header_seen = true;
            continue; // header row
        }
        if is_separator(&cells) {
            continue;
        }
        match read_row(&cells) {
            Ok(row) => rows.push(row),
            Err(why) => problems.push(format!("line {line_no}: {why}")),
        }
    }
    if !header_seen {
        problems.push(format!("no {ROADMAP_COLUMNS}-column table found"));
    }
    if problems.is_empty() {
        RoadmapShape::WellFormed { rows }
    } else {
        RoadmapShape::Malformed { problems }
    }
}

/// Reads one body row, or says in one clause what is wrong with it.
pub(crate) fn read_row(cells: &[String]) -> Result<RoadmapRow, String> {
    let (
        Some(index_cell),
        Some(item_cell),
        Some(weight_cell),
        Some(needs_cell),
        Some(status_cell),
        Some(evidence_cell),
    ) = (
        cells.first(),
        cells.get(1),
        cells.get(2),
        cells.get(3),
        cells.get(4),
        cells.get(5),
    )
    else {
        return Err("the row is short of cells".to_owned());
    };
    let id = NodeId::parse(index_cell).map_err(|err| err.subject().to_owned())?;
    let weight = if weight_cell.is_empty() {
        1
    } else {
        weight_cell
            .parse::<u32>()
            .map_err(|_| format!("weight `{weight_cell}` is not a number"))?
    };
    let mut needs = Vec::new();
    for part in needs_cell.split(',') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        needs.push(NodeId::parse(trimmed).map_err(|err| err.subject().to_owned())?);
    }
    let status = parse_status(status_cell)
        .ok_or_else(|| format!("status `{status_cell}` outside the five-value enum"))?;
    let evidence = if evidence_cell.is_empty() {
        EvidenceCell::Empty
    } else {
        match Locator::parse(evidence_cell) {
            Ok(locator) => EvidenceCell::Present(locator),
            Err(_) => EvidenceCell::Invalid {
                raw: evidence_cell.clone(),
            },
        }
    };
    Ok(RoadmapRow {
        id,
        item: item_cell.clone(),
        weight,
        needs,
        status,
        evidence,
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use super::super::rewrite::set_roadmap_status;
    use super::*;
    use crate::error::AxCode;
    use crate::locator::Locator;
    use crate::node_id::NodeId;
    fn node(raw: &str) -> NodeId {
        NodeId::parse(raw).unwrap()
    }
    fn locator() -> Locator {
        Locator::parse(&format!("cas:b3-{}", "ab".repeat(32))).unwrap()
    }
    fn rows_of(text: &str) -> Vec<RoadmapRow> {
        match check_roadmap_shape(text) {
            RoadmapShape::WellFormed { rows } => rows,
            RoadmapShape::Malformed { problems } => panic!("expected well-formed: {problems:?}"),
        }
    }

    const GOOD: &str = "# Roadmap

| # | Item | Weight | Needs | Status | Evidence |
|---|------|--------|-------|--------|----------|
| 1 | chain verification | 1 |  | Done | cas:b3-abababababababababababababababababababababababababababababababab |
| 2 | range retrieval | 3 | 1 | In progress |  |
| 3 | fixtures | 1 |  | Awaiting approval |  |
| 4 | claimed without evidence | 1 |  | Done |  |
| 5 | evidence that does not parse | 1 |  | done | not-a-locator |
";
    #[test]
    fn a_good_table_parses_every_column() {
        let rows = rows_of(GOOD);
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0].status, RoadmapStatus::Done);
        assert!(matches!(rows[0].evidence, EvidenceCell::Present(_)));
        assert_eq!(rows[1].weight, 3);
        assert_eq!(rows[1].needs, vec![node("1")]);
        assert_eq!(rows[2].weight, 1, "an empty weight cell reads as one");
        assert!(rows[2].needs.is_empty());
        assert!(matches!(rows[3].evidence, EvidenceCell::Empty));
        assert!(matches!(rows[4].evidence, EvidenceCell::Invalid { .. }));
        assert_eq!(rows[4].status, RoadmapStatus::Done, "case is not contract");
    }

    #[test]
    fn done_without_evidence_is_refused_where_it_is_written() {
        let refusal = set_roadmap_status(GOOD, &node("2"), RoadmapStatus::Done, None).unwrap_err();
        assert_eq!(refusal.code(), &AxCode::EvidenceMissing);
        assert!(refusal.recovery().contains("cas:"));
        let text =
            set_roadmap_status(GOOD, &node("2"), RoadmapStatus::Done, Some(&locator())).unwrap();
        assert!(text.contains("| 2 | range retrieval | 3 | 1 | Done | cas:b3-"));
    }

    #[test]
    fn an_index_no_row_carries_is_refused_by_number() {
        let refusal =
            set_roadmap_status(GOOD, &node("99"), RoadmapStatus::InProgress, None).unwrap_err();
        assert_eq!(refusal.code(), &AxCode::InvalidArgs);
        assert!(refusal.subject().contains("99"));
    }

    #[test]
    fn a_second_table_below_the_roadmap_is_not_edited() {
        let text = format!(
            "{GOOD}
## Notes

| # | Item | Weight | Needs | Status | Evidence |
|---|---|---|---|---|---|
| 2 | other table | 1 |  | Not started |  |
"
        );
        let edited = set_roadmap_status(&text, &node("2"), RoadmapStatus::Blocked, None).unwrap();
        assert!(
            edited.contains("| 2 | other table | 1 |  | Not started |  |"),
            "only the first table is the roadmap"
        );
        assert!(edited.contains("| 2 | range retrieval | 3 | 1 | Blocked |  |"));
    }

    #[test]
    fn wrong_column_count_and_alien_status_are_named_problems() {
        let text = "\
| # | Item | Status |
|---|------|--------|
| 1 | x | nearly there |
";
        let RoadmapShape::Malformed { problems } = check_roadmap_shape(text) else {
            panic!("expected malformed");
        };
        assert!(problems.iter().any(|p| p.contains("3 columns")));
    }

    #[test]
    fn free_text_status_is_rejected_by_the_enum() {
        let text = "\
| # | Item | Weight | Needs | Status | Evidence |
|---|------|--------|-------|--------|----------|
| 1 | x | 1 |  | nearly there |  |
";
        let RoadmapShape::Malformed { problems } = check_roadmap_shape(text) else {
            panic!("expected malformed");
        };
        assert!(problems.iter().any(|p| p.contains("nearly there")));
    }

    /// The four-column table is the old grammar, and a plan written in
    /// it has no weights and no dependencies. It is reported rather than
    /// half-read: reading four columns as six would put the status word
    /// in the weight cell.
    /// The four-column table is the old grammar, and a plan written in
    /// it has no weights and no dependencies. It is reported rather than
    /// half-read: reading four columns as six would put the status word
    /// in the weight cell.
    #[test]
    fn the_old_four_column_table_is_reported_rather_than_half_read() {
        let old = "\
| # | Item | Status | Evidence |
|---|------|--------|----------|
| 1 | x | Not started | |
";
        let RoadmapShape::Malformed { problems } = check_roadmap_shape(old) else {
            panic!("expected malformed");
        };
        assert!(problems.iter().any(|p| p.contains("4 columns")));
    }

    #[test]
    fn a_bad_index_and_a_bad_weight_are_named_by_line() {
        let text = "\
| # | Item | Weight | Needs | Status | Evidence |
|---|------|--------|-------|--------|----------|
| one | x | 1 |  | Not started |  |
| 2 | y | heavy |  | Not started |  |
| 3 | z | 1 | nine | Not started |  |
";
        let RoadmapShape::Malformed { problems } = check_roadmap_shape(text) else {
            panic!("expected malformed");
        };
        assert_eq!(problems.len(), 3);
        assert!(problems[0].contains("line 3"));
        assert!(problems[1].contains("heavy"));
        assert!(problems[2].contains("line 5"));
    }

    #[test]
    fn no_table_at_all_is_malformed() {
        assert!(matches!(
            check_roadmap_shape("just prose"),
            RoadmapShape::Malformed { .. }
        ));
    }
}

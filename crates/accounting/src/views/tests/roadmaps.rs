// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::founded;
use crate::views::Views;

#[test]
fn a_roadmap_counts_only_the_rows_that_carry_evidence() {
    let dir = tempfile::tempdir().unwrap();
    founded(dir.path());
    let building = dir.path().join("lab");
    std::fs::create_dir_all(&building).unwrap();
    let evidence = format!("cas:b3-{}", "ab".repeat(32));
    std::fs::write(
        building.join("Roadmap.md"),
        format!(
            "# Roadmap\n\n| # | Item | Weight | Needs | Status | Evidence |\n\
             |---|---|---|---|---|---|\n\
             | 1 | wired | 1 |  | Done | {evidence} |\n\
             | 2 | claimed | 1 |  | Done |  |\n\
             | 3 | waiting | 1 |  | Awaiting approval |  |\n\
             | 4 | later | 1 |  | not started |  |\n"
        ),
    )
    .unwrap();

    let mut views = Views::new(dir.path());
    let wire::Answer::City(city) = views.answer(&wire::Query::CityView) else {
        panic!("CityView answers with a city");
    };
    // Two buildings: City Hall, which every city is raised with, and
    // the one this test wrote a plan for.
    let plan = city
        .buildings
        .iter()
        .find(|found| found.addr.as_str() == "lab")
        .expect("the building this test wrote a plan for");
    assert!(plan.problems.is_empty(), "{:?}", plan.problems);
    let kernel::Progress::Planned(planned) = plan.progress else {
        panic!("a building with a roadmap has a denominator");
    };
    // Four rows; one Done with evidence counts; the evidence-free Done
    // stays visible and out of the numerator; awaiting approval reads
    // as blocked; `not started` proves case is not part of the contract.
    assert_eq!(
        (planned.done, planned.blocked, planned.total),
        (1, 1, 4),
        "{planned:?}"
    );
}

#[test]
fn a_roadmap_that_cannot_be_parsed_reports_its_rows_rather_than_a_number() {
    let dir = tempfile::tempdir().unwrap();
    founded(dir.path());
    let building = dir.path().join("lab");
    std::fs::create_dir_all(&building).unwrap();
    std::fs::write(
        building.join("Roadmap.md"),
        "| # | Item | Weight | Needs | Status | Evidence |\n\
         |---|---|---|---|---|---|\n\
         | 1 | x | 1 |  | nearly there |  |\n",
    )
    .unwrap();

    let mut views = Views::new(dir.path());
    let wire::Answer::City(city) = views.answer(&wire::Query::CityView) else {
        panic!("CityView answers with a city");
    };
    let plan = city
        .buildings
        .iter()
        .find(|found| found.addr.as_str() == "lab")
        .expect("the building this test wrote a plan for");
    assert!(plan.problems.iter().any(|p| p.contains("nearly there")));
    assert!(
        matches!(plan.progress, kernel::Progress::Unplanned(_)),
        "an unreadable plan has no denominator, and no percentage"
    );
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a city can do, as the settings page lays it out: the folder a
// skill is dropped into, the buildings down the left, and the shelves
// that building reads from.
//
// **The rows are what `city::Library` answers, one per name.** A library
// is keyed by name, so a skill the city keeps and the same name on the
// building's own shelf are one row - the nearer shelf, which is what
// `Library::scan` writes last - and no answer can hold a name twice.
// Within a shelf the catalog lists section then name, and the shelves
// themselves come in `Shelf::catalog_position` order: the city's stock,
// the building's own shelf, then the directories the city mounts.
//
// The rows cover the readings that differ on screen: a holding the
// reading room admits and runs have used, one on the shelf that nothing
// admits, a name whose building's own copy is the one that wins, a name
// mounted from a directory outside the city, and a name the room asks
// for that no shelf has.
//
// A file of its own rather than another case among the settings
// screens, because the answer below is the fixture: it is read beside
// the screen it stands up, not four hundred lines away from it.

import { Address, B3Hash, RunId } from "../../wire";
import type { Answer, Query, SkillsAnswer } from "../../wire";
import { SkillsSection } from "../setup/skills";
import { Case } from "./case";
import { Stand } from "./stand";

const HALL = Address.make("hall");

// Two runs were frozen with the first holding pinned; nothing has read
// the other two. The ids are the shape a run id has, so the row counts
// what a real answer would count.
const EARLIER = RunId.make("0199c0de-1a2b-4c3d-8e4f-5a6b7c8d9e0f");
const LATER = RunId.make("0199c0de-1a2b-4c3d-8e4f-5a6b7c8d9e10");

const SHELVED: SkillsAnswer = {
  building: HALL,
  skills: [
    {
      name: "diagnosing-bugs",
      section: "engineering",
      // A holding on the city's stock: its address is a path under the
      // city root, which is what `Library::address_of` produces.
      shelf: { library: Address.make(".sprawling/library/engineering/diagnosing-bugs.md") },
      disclosure: "Find the defect from the evidence, then write the test that would have caught it.",
      hash: B3Hash.make("af3c1d2e4b5a69780c1d2e3f4a5b6c7d8e9f0a1b2c3d4e5f60718293a4b5c6d7"),
      admitted: true,
      pinned_by: [EARLIER, LATER],
    },
    {
      name: "resolving-merge-conflicts",
      section: "engineering",
      shelf: { library: Address.make(".sprawling/library/engineering/resolving-merge-conflicts.md") },
      disclosure: "Replay both sides against the tests before choosing either.",
      hash: B3Hash.make("b1c2d3e4f5061728394a5b6c7d8e9f0a1b2c3d4e5f60718293a4b5c6d7e8f901"),
      admitted: false,
      pinned_by: [],
    },
    {
      // The nearer shelf wins under one name, so the library's own copy
      // of this name is not a second row: it is the row the run would
      // read, one shelf closer.
      name: "kiln-firing",
      section: "hall",
      shelf: { building: Address.make("hall/.sprawling/skills/hall/kiln-firing.md") },
      disclosure: "Fire the kiln at the curve this building agreed on, not the one the book prints.",
      hash: B3Hash.make("c2d3e4f5061728394a5b6c7d8e9f0a1b2c3d4e5f60718293a4b5c6d7e8f90102"),
      admitted: true,
      pinned_by: [],
    },
    {
      name: "apostle-antislop",
      section: "",
      // The arm a running client cannot reach yet, and the reason this
      // route exists: an external shelf has no city address, so the row
      // has to name where it came from and refuse to open. Its path is
      // the skill's directory and the one file name
      // `Library::shelve_external` reads, and its section is empty
      // because that tree has no section level.
      shelf: { external: { index: 0, path: "apostle-antislop/SKILL.md" } },
      disclosure: "Edit prose that has gone model-shaped rather than subject-shaped.",
      hash: B3Hash.make("d3e4f5061728394a5b6c7d8e9f0a1b2c3d4e5f60718293a4b5c6d7e8f9010203"),
      admitted: true,
      pinned_by: [],
    },
  ],
  missing: ["apostle-translation"],
};

// The city answers one question here: the shelves. Everything else the
// section asks - which buildings there are, and the document behind a
// row - is unanswered, which is the state a page holds while it waits.
function answered(query: Query): Answer | undefined {
  return typeof query === "object" && "skills" in query ? { skills: SHELVED } : undefined;
}

export function Shelved() {
  return (
    <Case label="settings · the shelves one building reads from">
      <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answered}>
        <SkillsSection />
      </Stand>
    </Case>
  );
}

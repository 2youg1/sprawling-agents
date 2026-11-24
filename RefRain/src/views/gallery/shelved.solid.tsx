// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Answer, Query, SkillsAnswer } from "../../wire";
import { Address, B3Hash, RunId } from "../../wire";
const HALL = Address.make("hall");
const EARLIER = RunId.make("0199c0de-1a2b-4c3d-8e4f-5a6b7c8d9e0f");
const LATER = RunId.make("0199c0de-1a2b-4c3d-8e4f-5a6b7c8d9e10");
export const SHELVED: SkillsAnswer = {
  building: HALL,
  external_shelves: ["~/.claude/skills"],
  skills: [
    {
      name: "diagnosing-bugs",
      section: "engineering",
      shelf: "library",
      disclosure:
        "Find the defect from the evidence, then write the test that would have caught it.",
      hash: B3Hash.make("af3c1d2e4b5a69780c1d2e3f4a5b6c7d8e9f0a1b2c3d4e5f60718293a4b5c6d7"),
      admitted: true,
      pinned_by: [EARLIER, LATER],
    },
    {
      name: "resolving-merge-conflicts",
      section: "engineering",
      shelf: "library",
      disclosure: "Replay both sides against the tests before choosing either.",
      hash: B3Hash.make("b1c2d3e4f5061728394a5b6c7d8e9f0a1b2c3d4e5f60718293a4b5c6d7e8f901"),
      admitted: false,
      pinned_by: [],
    },
    {
      name: "kiln-firing",
      section: "hall",
      shelf: { building: Address.make("hall/.sprawling/skills/hall/kiln-firing.md") },
      disclosure:
        "Fire the kiln at the curve this building agreed on, not the one the book prints.",
      hash: B3Hash.make("c2d3e4f5061728394a5b6c7d8e9f0a1b2c3d4e5f60718293a4b5c6d7e8f90102"),
      admitted: true,
      pinned_by: [],
    },
    {
      name: "apostle-antislop",
      section: "",
      shelf: { external: { index: 0, path: "apostle-antislop/SKILL.md" } },
      disclosure: "Edit prose that has gone model-shaped rather than subject-shaped.",
      hash: B3Hash.make("d3e4f5061728394a5b6c7d8e9f0a1b2c3d4e5f60718293a4b5c6d7e8f9010203"),
      admitted: true,
      pinned_by: [],
    },
  ],
  missing: ["apostle-translation"],
};
export function answered(query: Query): Answer | undefined {
  return typeof query === "object" && "skills" in query ? { skills: SHELVED } : undefined;
}
function bare(query: Query): Answer | undefined {
  return typeof query === "object" && "skills" in query
    ? { skills: { building: HALL, external_shelves: [], skills: [], missing: [] } }
    : undefined;
}
import SkillsSection from "../setup/skills_view";
import { GalleryCase } from "./case";
import Stand from "./stand.solid";

export default function SkillsCases() {
  return (
    <>
      <GalleryCase label="settings · the shelves one building reads from">
        <Stand
          link={{ kind: "live", city: "sprawling" }}
          unread={[]}
          waiting={[]}
          answers={answered}
          children={() => (
            <>
              <SkillsSection />
            </>
          )}
        />
      </GalleryCase>
      <GalleryCase label="settings · empty shelves say where a skill goes">
        <Stand
          link={{ kind: "live", city: "sprawling" }}
          unread={[]}
          waiting={[]}
          answers={bare}
          children={() => (
            <>
              <SkillsSection />
            </>
          )}
        />
      </GalleryCase>
    </>
  );
}

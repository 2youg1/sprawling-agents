// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What this city can do, and where a person puts one more of it: the
// buildings on the left, that building's shelves in the middle, and
// the skill a person opened on the right.
//
// **A shelf is read here and written on disk.** The city's library
// sits under the reserved prefix, which no write domain reaches - a
// resident may read the stock and may not restock it (`city::Library`)
// - so the wire has a question for the shelves and no frame that puts
// a document on one. The two folder paths are therefore part of the
// screen rather than a footnote: they are how a person adds a skill,
// and the list beside them is how they see it arrive.
//
// **Both the list and the reader are the building page's.** A second
// drawing of a shelf row, or of a document, would be a second answer
// to "what does this city have" the first time one of them was
// corrected.

import { Show, createSignal } from "solid-js";

import type { Address } from "../../wire";
import { useSay, useUi } from "../../ui";
import { Button } from "../parts/button";
import { EmptyState } from "../parts/empty";
import { FileView } from "../building/file";
import { Skills } from "../building/skills";
import { BuildingColumn, HALL, useBuildings } from "../shared/buildings";

// Where a person drops a Markdown file to give the whole city one more
// skill, and the copy button that puts it on the clipboard. The city
// answers with its own name, so what is drawn is the folder as it is on
// this machine rather than a shape to be filled in.
//
// The welcome walk shows this much on its own, because a person who
// has just built a city has nothing on the shelves to list yet.
export function Shelves() {
  const ui = useUi();
  const say = useSay();
  const path = () => `${ui.conn.belief.city ?? "<city>"}/.sprawling/library/`;
  return (
    <div class="flex flex-col gap-snug text-note text-text-quiet">
      <p>{say("setup_skills")}</p>
      <div class="flex items-center gap-snug">
        <code class="rounded-control bg-raised px-base py-snug font-mono text-text">{path()}</code>
        <Button
          label={say("setup_copy")}
          tone="quiet"
          onPress={() => {
            void navigator.clipboard.writeText(path());
          }}
        />
      </div>
      <p class="text-text-faint">{say("setup_skills_note")}</p>
    </div>
  );
}

export function SkillsSection() {
  const say = useSay();
  const buildings = useBuildings();
  const [chosen, setChosen] = createSignal<Address>(HALL);
  // The skill whose document is open, or nothing when the person has
  // opened none. Cleared when the building changes, because a document
  // from one building's shelf under another building's list is a claim
  // about where that skill sits.
  const [reading, setReading] = createSignal<Address | null>(null);

  return (
    <div class="flex min-w-0 flex-col gap-wide">
      <Shelves />
      <div class="flex min-w-0 gap-wide">
        <BuildingColumn
          label={say("mcp_building")}
          buildings={buildings()}
          chosen={chosen()}
          hall={say("city_hall")}
          onPick={(addr) => {
            setChosen(addr);
            setReading(null);
          }}
        />
        <div class="flex min-w-0 flex-1 flex-col gap-wide">
          <Skills
            building={chosen()}
            onPick={(picked) => {
              setReading(picked.at);
            }}
          />
          <Show
            when={reading()}
            fallback={<EmptyState text={say("skills_unopened")} />}
          >
            {(at) => <FileView at={at()} root={chosen()} />}
          </Show>
        </div>
      </div>
    </div>
  );
}

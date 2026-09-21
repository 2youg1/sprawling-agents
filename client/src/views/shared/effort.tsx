// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// How hard the city thinks, and the three things a person has to know
// before they choose: the choice is frozen for the length of a run,
// the two wire formats disagree about where `none` is written, and
// saying nothing is not the same as saying `none`.
//
// The paragraph states all three; the line under the track states what
// the level now held costs. The words are in `lang.json` like every
// other word a reader is handed - this file only says where they go.
//
// The welcome walk and the settings page both show this, so the
// paragraph cannot be present on one page and missing from the other.
// The track used to live beside the model table, which is the other
// question that screen asks; it is here now because how hard the city
// thinks is not a fact about which model does the thinking.

import { EFFORTS } from "../../core/prefs";
import { UNSTATED } from "../../core/slash";
import type { Effort } from "../../wire";
import { useSay, useUi } from "../../ui";
import { Segmented, type Choice } from "../parts/segmented";

// The cell for a person who has not chosen, which is the state a new
// city is in. It leads the track because that is where everybody
// starts, and no level can stand in for it: `none` asks the provider
// to think as little as it can, while saying nothing leaves the
// choice to the provider.
//
// What the city keeps stays `Effort | null`. The word is a label this
// track puts on at the edge that draws a cell and takes off at the
// edge that writes the choice, so absence keeps its one spelling in
// `core/prefs.ts`. It is imported rather than written again because a
// person reaches the same cell by typing `/effort unstated`, and the
// two have to be the same word.
type Level = Effort | typeof UNSTATED;

export function EffortChoice() {
  const ui = useUi();
  const say = useSay();
  const cells = (): readonly Choice<Level>[] => [
    { value: UNSTATED, label: say("effort_unstated") },
    ...EFFORTS.map((effort) => ({ value: effort, label: say(`effort_${effort}`) })),
  ];
  return (
    <div class="flex flex-col gap-tight text-note text-text-quiet">
      {say("setup_effort")}
      <Segmented<Level>
        label={say("setup_effort")}
        options={cells()}
        held={ui.prefs.effort() ?? UNSTATED}
        onPick={(level) => {
          ui.prefs.setEffort(level === UNSTATED ? null : level);
        }}
      />
    </div>
  );
}

export function EffortSection() {
  const ui = useUi();
  const say = useSay();
  // Nobody having chosen is a state of its own, and it is the state a
  // new city is in: `core/prefs.ts` answers `null`, the request omits
  // the field, and the provider decides. Reading it as `none` here
  // would tell a person their city had been asked not to think.
  const note = () => {
    const held = ui.prefs.effort();
    return held === null ? say("effort_note_unstated") : say(`effort_note_${held}`);
  };
  return (
    <div class="flex flex-col gap-base">
      <EffortChoice />
      <p class="max-w-measure text-note leading-relaxed text-text-faint">{say("setup_effort_essay")}</p>
      <p class="max-w-measure text-note text-text-quiet">{note()}</p>
    </div>
  );
}

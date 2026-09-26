<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The sliding chooser in the six states it can be drawn in: two
  // cells, three cells, three cells under two group headings, a cell
  // that cannot be chosen and says why, a control nobody has answered
  // yet, and the whole control with motion turned off.
  //
  // It is a file of its own rather than six more cases among the shared
  // controls because each fixture holds the choice a person made in it,
  // and six more signals is what would push that component past
  // reading in one screen.

  import type { Choice, Group } from "../parts/segmented";

  // The two laboratories the three dialects come from, and the colour
  // each is given. The sliding chooser is the only control in the
  // client that paints by family, and the mapping is spelled at the
  // call site because the control itself knows no vendor's name.
  //
  // wording-ok: the laboratory names are proper nouns - the same
  // letters in every language - so they are written here rather than
  // in `lang.json`.
  const OPEN_AI: Group = { label: "OpenAI", tone: "plain" };
  const ANTHROPIC: Group = { label: "Anthropic", tone: "alert" };
</script>

<script lang="ts">
  import { WIRE_APIS } from "../../core/commands";
  import type { WireApi } from "../../core/commands";
  import { say } from "../../core/lang";
  import type { Chroma, Lighting, Motion } from "../../core/appearance";
  import { ui } from "../../ui";
  import Segmented from "../parts/segmented.svelte";
  import Case from "./case.svelte";

  const { lang } = ui();

  let chroma = $state<Chroma>("full");
  let lighting = $state<Lighting>("system");
  let motion = $state<Motion>("off");
  let wire = $state<WireApi>("chat");
  let gated = $state<WireApi>("chat");
  let unchosen = $state<WireApi | null>(null);

  // The dialect list the provider form offers, grouped by the
  // laboratory whose wire it speaks. The words are the literal values
  // of Codex's `wire_api` key, which no language translates.
  function dialects(): readonly Choice<WireApi>[] {
    return WIRE_APIS.map((api) => ({
      value: api,
      label: api,
      group: api === "messages" ? ANTHROPIC : OPEN_AI,
    }));
  }

  // The same list as this city can serve it today: the middle dialect
  // is named, drawn and refused, so the reason arrives under the
  // pointer instead of after the click.
  function carried(): readonly Choice<WireApi>[] {
    return dialects().map((choice) =>
      choice.value === "responses"
        ? { ...choice, why: say($lang, "setup_wire_api_unsupported") }
        : choice,
    );
  }
</script>

<Case label="segmented · two cells">
  <Segmented
    label={say($lang, "appearance_chroma")}
    options={[
      { value: "full", label: say($lang, "appearance_chroma_full") },
      { value: "off", label: say($lang, "appearance_chroma_none") },
    ]}
    held={chroma}
    onPick={(value) => {
      chroma = value;
    }}
  />
</Case>

<Case label="segmented · three cells">
  <Segmented
    label={say($lang, "appearance_lighting")}
    options={[
      { value: "system", label: say($lang, "appearance_lighting_system") },
      { value: "dark", label: say($lang, "appearance_lighting_dark") },
      { value: "light", label: say($lang, "appearance_lighting_light") },
    ]}
    held={lighting}
    onPick={(value) => {
      lighting = value;
    }}
  />
</Case>

<Case label="segmented · three cells under two group headings">
  <Segmented
    label={say($lang, "setup_wire_api")}
    options={dialects()}
    held={wire}
    onPick={(value) => {
      wire = value;
    }}
  />
</Case>

<Case label="segmented · a cell that cannot be chosen">
  <Segmented
    label={say($lang, "setup_wire_api")}
    options={carried()}
    held={gated}
    onPick={(value) => {
      gated = value;
    }}
  />
</Case>

<!-- A caller holding nothing passes `null`, and the track answers by
drawing no slider: no cell may report itself chosen when nobody has
chosen. The first cell a person can choose is still the one tab stop,
so the control is reachable by keyboard before it has an answer. -->
<Case label="segmented · nothing chosen yet">
  <Segmented
    label={say($lang, "setup_wire_api")}
    options={dialects()}
    held={unchosen}
    onPick={(value) => {
      unchosen = value;
    }}
  />
</Case>

<!-- The one subtree on this route that asks for motion to stop
(ux B6). Everything under the mark keeps its state changes and loses
its travel, so `xtask render` can read the same classes at rest here
and in flight in every case above. -->
<Case label="segmented · motion off">
  <div data-motion="off">
    <Segmented
      label={say($lang, "appearance_motion")}
      options={[
        { value: "system", label: say($lang, "appearance_motion_system") },
        { value: "on", label: say($lang, "appearance_motion_full") },
        { value: "off", label: say($lang, "appearance_motion_off") },
      ]}
      held={motion}
      onPick={(value) => {
        motion = value;
      }}
    />
  </div>
</Case>

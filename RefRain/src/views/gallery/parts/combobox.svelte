<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The combobox in each state it can be measured in: closed beside an
  // open one, open with a value in force and the cursor on another row,
  // and open at the bottom edge of a box that clips it, where it has to
  // open upward to be seen at all.
  import { say } from "../../../core/lang";
  import { ui } from "../../../ui";
  import Combobox from "../../parts/combobox.svelte";
  import Case from "../case.svelte";
  import { CHOSEN, MODELS } from "../served";

  const { lang } = ui();

  let model = $state<string | null>(null);
  let held = $state<string | null>(MODELS.at(-1)?.id ?? CHOSEN.id);
  let edge = $state<string | null>(CHOSEN.id);
  let face = $state<string | null>("Georgia");
  const choices = MODELS.map((each) => ({ value: each.id, label: each.id, note: each.context }));
  // Each row wears its own family so the appearance group's local-face
  // picker can be measured: the label is the family name drawn in that face.
  const faces = [
    { value: "Georgia", label: "Georgia", face: '"Georgia", ui-serif, serif' },
    { value: "Courier New", label: "Courier New", face: '"Courier New", ui-monospace, monospace' },
    {
      value: "Trebuchet MS",
      label: "Trebuchet MS",
      face: '"Trebuchet MS", ui-sans-serif, sans-serif',
    },
  ];
</script>

<!-- Stacked as the settings page stacks them: the open list covers the second trigger.
The list is positioned and takes no room, so `pb-output` (the list's own max height) grows the section to hold it. -->
<Case label="combobox · open on click, nothing chosen">
  <div class="flex flex-col gap-base pb-output">
    <Combobox
      label={say($lang, "setup_main")}
      placeholder={say($lang, "part_search")}
      empty={say($lang, "part_no_match")}
      {choices}
      value={model}
      onPick={(value) => {
        model = value;
      }}
      starts="open"
    />
    <Combobox
      label={say($lang, "setup_digest")}
      placeholder={say($lang, "part_search")}
      empty={say($lang, "part_no_match")}
      {choices}
      value={model}
      onPick={(value) => {
        model = value;
      }}
    />
  </div>
</Case>

<!-- The value in force is the deeper wash on the last row; the cursor
starts on the first, where its bar stands at the leading edge. -->
<Case label="combobox · a value in force, the cursor on another row">
  <div class="pb-output">
    <Combobox
      label={say($lang, "setup_main")}
      placeholder={say($lang, "part_search")}
      empty={say($lang, "part_no_match")}
      {choices}
      value={held}
      onPick={(value) => {
        held = value;
      }}
      starts="open"
    />
  </div>
</Case>

<!-- A box as tall as a palette that cuts what it holds, with the trigger
in its bottom-right corner: below the trigger there is no room, so the
list opens upward and stays inside the box. Opened downward, it would be
cut away whole, and `xtask render` would find a list with no option
showing. -->
<Case label="combobox · in the bottom corner of a box that clips it, opening upward">
  <div class="flex h-palette flex-col items-end justify-end overflow-hidden">
    <div class="w-tree">
      <Combobox
        label={say($lang, "setup_main")}
        placeholder={say($lang, "part_search")}
        empty={say($lang, "part_no_match")}
        {choices}
        value={edge}
        onPick={(value) => {
          edge = value;
        }}
        starts="open"
      />
    </div>
  </div>
</Case>

<!-- A face picker: every row is drawn in the family it names, so two
neighbours can be told apart by eye. The families are stock faces every
engine ships, so the fixture does not need Local Font Access. -->
<Case label="combobox · each row wears the face it names">
  <div class="pb-output">
    <Combobox
      label={say($lang, "appearance_local")}
      placeholder={say($lang, "part_search")}
      empty={say($lang, "part_no_match")}
      choices={faces}
      value={face}
      onPick={(value) => {
        face = value;
      }}
      starts="open"
    />
  </div>
</Case>

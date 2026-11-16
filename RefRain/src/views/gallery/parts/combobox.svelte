<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
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

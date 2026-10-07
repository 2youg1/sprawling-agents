<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // A building's `RULES.toml`, read and written whole (refrain roadmap
  // UG, W6's `PutRules`). The city reads the text before it lands and
  // refuses one it cannot read, and refuses a write made against text
  // the Mayor has changed since this box read it: the draft stays in the
  // box either way, and saving again writes against what is there now.
  // This page writes no TOML of its own and checks none: the city is the
  // one reader of this grammar. The door is the person's at this
  // machine; a remote page is refused by the city, and the refusal is
  // what it reads.

  import { putRules } from "../../core/commands";
  import { readDocument } from "../../core/document";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Address } from "../../wire";
  import { HALL, useBuildings } from "../shared/buildings";
  import Card from "./card.svelte";
  import { rulesAt } from "./files";
  import { HELD, answered, awaitReceipt, edited, refused, sent } from "./saving";
  import type { Saving, Slot } from "./saving";

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;
  const buildings = useBuildings();

  let chosen = $state<Address>(HALL);
  const held = $derived(u.conn.asking.ask({ document: { at: rulesAt(chosen) } }));
  // A building that has no rules file yet reads as empty text, and the
  // first save writes it.
  const onDisk = $derived.by(() => {
    const read = readDocument($held);
    return read.kind === "held" ? read.value.text : read.kind === "unavailable" ? "" : null;
  });

  let draft = $state("");
  let saving = $state.raw<Saving>(HELD);
  const slot: Slot = { now: () => saving, mark: (next) => (saving = next) };

  // The box follows the file until somebody types, and again once their
  // text has landed or another building is chosen.
  let was: Address | null = null;
  $effect(() => {
    const at = chosen;
    if (onDisk === null) return;
    saving = answered(saving, onDisk);
    if (was !== at || saving.kind === "held" || saving.kind === "saved") draft = onDisk;
    if (was !== at) saving = HELD;
    was = at;
  });
  let seen = $belief.refusal;
  $effect(() => {
    const error = $belief.refusal;
    if (error === null || error === seen) return;
    seen = error;
    saving = refused(saving, error);
  });

  function save(): void {
    if (onDisk === null || !u.send(putRules(chosen, onDisk, draft))) return;
    const mine = sent(onDisk);
    saving = mine;
    awaitReceipt(mine, slot);
  }
</script>

<!-- The building is picked in a native <select> rather than the column
     the skills and MCP pages draw (client D78): a column of one row reads
     as a label, so a city with only the hall showed nothing to open,
     while a select is a control with one option or with fifty, and the
     platform owns its keys. -->
<div class="flex flex-col gap-base">
  <select
    class="h-control w-tree max-w-full rounded-control border border-edge-input bg-raised px-base text-body text-text"
    aria-label={say($lang, "rules_building")}
    value={chosen}
    onchange={(event) => {
      const picked = $buildings.find((addr) => addr === event.currentTarget.value);
      if (picked !== undefined) chosen = picked;
    }}
  >
    {#each $buildings as addr (addr)}
      <option value={addr}>{addr === HALL ? say($lang, "city_hall") : addr}</option>
    {/each}
  </select>
  <Card title="rules_title" note="rules_note" {saving} settled="settings_next_run" onSave={save}>
    <textarea
      class="min-h-output w-full rounded-control border border-edge-input bg-page px-base py-snug font-mono text-note text-text"
      aria-label={say($lang, "rules_title")}
      spellcheck="false"
      bind:value={draft}
      oninput={() => {
        saving = edited(draft !== onDisk);
      }}
    ></textarea>
    <code class="truncate font-mono text-note text-text-faint">{rulesAt(chosen)}</code>
  </Card>
</div>

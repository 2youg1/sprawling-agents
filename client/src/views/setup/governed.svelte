<script lang="ts" module>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The three documents that govern the city - who the mayor is, who the
// clerk is, and how this person wants the city run - each edited whole
// in one box and saved once, the way the city writes them
// (`city::governed`). The page reads the file where the city keeps it,
// in the reserved subtree at the city's root, and saves through
// `PutDocument`, which names the document rather than the path.

import type { GovernedDocument } from "../../wire";
import { Address } from "../../wire";

const FILES: Record<GovernedDocument, string> = {
  mayor: "MAYOR.md",
  clerk: "CLERK.md",
  preferences: "PREFERENCES.md",
};

export const GOVERNED: readonly GovernedDocument[] = ["mayor", "clerk", "preferences"];

export function governedAt(which: GovernedDocument): Address {
  return Address.make(`.sprawling/${FILES[which]}`);
}
</script>

<script lang="ts">
  import { untrack } from "svelte";

  import { readAnswer } from "../../core/answered";
  import { putDocument } from "../../core/commands";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Query } from "../../wire";
  import Button from "../parts/button.svelte";
  import Segmented from "../parts/segmented.svelte";
  import Unanswered from "../parts/unanswered.svelte";

  const u = ui();
  const lang = u.lang;

  let which = $state<GovernedDocument>("mayor");
  let draft = $state("");
  let edited = $state(false);

  const question = $derived<Query>({ document: { at: governedAt(which) } });
  const held = $derived(u.conn.asking.ask(question));
  // A document the city has not written yet answers `unavailable`, as an
  // unreadable one does; either way the box starts empty and the page
  // says it could not read one, and a save writes the file whole.
  const read = $derived(readAnswer($held, (answer) => ("document" in answer ? answer.document.text : undefined)));
  const onDisk = $derived(read.kind === "held" ? read.value : "");

  // The box follows the file until somebody types in it, and follows it
  // again once their text has landed or another document is chosen.
  let was: GovernedDocument | undefined = undefined;
  $effect(() => {
    const at = which;
    const text = onDisk;
    if (!untrack(() => edited) || was !== at) {
      edited = false;
      draft = text;
    }
    was = at;
  });

  function save(): void {
    if (u.send(putDocument(which, onDisk, draft))) {
      edited = false;
    }
  }
</script>

<div class="flex flex-col gap-tight rounded-card bg-raised px-base py-snug">
  <span class="text-label font-label text-text">{say($lang, "setup_governed")}</span>
  <p class="text-note text-text-faint">{say($lang, "setup_governed_note")}</p>
  <Segmented
    label={say($lang, "setup_governed")}
    options={GOVERNED.map((each) => ({ value: each, label: say($lang, `governed_${each}`) }))}
    held={which}
    onPick={(next: GovernedDocument) => {
      which = next;
    }}
  />
  <textarea
    class="min-h-output w-full rounded-control bg-chrome px-base py-snug font-mono text-note text-text"
    aria-label={say($lang, `governed_${which}`)}
    bind:value={draft}
    oninput={() => {
      edited = true;
    }}
  ></textarea>
  {#if read.kind === "unavailable"}
    <Unanswered query={read.query} asked={question} />
  {/if}
  <div class="flex items-center gap-base">
    <Button
      label={say($lang, "setup_governed_save")}
      tone="primary"
      {...(edited ? {} : { why: say($lang, "setup_governed_unchanged") })}
      onPress={save}
    />
    <span class="flex-1"></span>
    <code class="truncate font-mono text-note text-text-faint">{governedAt(which)}</code>
  </div>
</div>

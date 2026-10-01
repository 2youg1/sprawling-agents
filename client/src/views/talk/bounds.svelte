<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // What bounds a run in this room, as two facts on the composer's
  // settings row (client-SPEC 7D, 7I): who answers the gate, and the
  // sandbox the room's building boxes a run in. They are read, never
  // pressed.
  //
  // **Both are risk readings, so the worrying value is marked rather than
  // left to be noticed.** No sandbox named anywhere means the run has the
  // machine, and a gate the city could not report is not a gate a person
  // can trust; either one carries the mark this product uses for anything
  // that needs a person (`asks`, client-SPEC 7C), which survives a forced
  // colour mode where a colour alone would not.
  import { readAnswer } from "../../core/answered";
  import { QUERIES } from "../../core/asking";
  import { fill, say } from "../../core/lang";
  import { buildingOf } from "../../core/route";
  import { ui } from "../../ui";
  import type { Address, Answer } from "../../wire";
  import Glyph from "../parts/glyph.svelte";
  import { FACT } from "./pill.svelte";

  interface Props {
    readonly room: Address | null;
  }

  const { room }: Props = $props();

  const u = ui();
  const { lang } = u;
  const governance = u.conn.asking.ask(QUERIES.governance);

  let building = $state<Answer | undefined>(undefined);
  $effect(() => {
    const at = room === null ? null : buildingOf(room);
    building = undefined;
    if (at === null) return;
    return u.conn.asking.ask({ building_view: { addr: at } }).subscribe((held) => {
      building = held;
    });
  });

  interface Bound {
    readonly words: string;
    readonly worrying: boolean;
  }

  const gate = $derived.by((): Bound => {
    const read = readAnswer($governance, (held) => ("governance" in held ? held.governance : undefined));
    switch (read.kind) {
      case "asking":
        return { words: "—", worrying: false };
      case "unavailable":
        return { words: say($lang, "facts_unreadable"), worrying: true };
      case "held":
        return {
          words: read.value.autonomy === "owner" ? say($lang, "autonomy_owner") : say($lang, "autonomy_clerk"),
          worrying: false,
        };
    }
  });

  const sandbox = $derived.by((): Bound => {
    const held = building;
    if (held === undefined || !("building" in held)) return { words: "—", worrying: false };
    const limits = held.building.sandbox;
    if (limits === null || limits === undefined) return { words: say($lang, "facts_sandbox_open"), worrying: true };
    return { words: fill(say($lang, "facts_sandbox_mounts"), { n: String(limits.mounts.length) }), worrying: false };
  });
</script>

<span class={[FACT, gate.worrying ? "asks" : ""]}>
  <Glyph name="gate" size="sm" />
  <span class="sr-only">{say($lang, "facts_autonomy")}</span>
  {gate.words}
</span>
<span class={[FACT, sandbox.worrying ? "asks" : ""]}>
  <Glyph name="sandbox" size="sm" />
  <span class="sr-only">{say($lang, "facts_sandbox")}</span>
  {sandbox.words}
</span>

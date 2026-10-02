<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // What bounds a run in this room, as two facts on the composer's
  // settings row (docs/frontend-method.md §7D, §7I): who answers the gate, and the
  // sandbox the room's building boxes a run in. They are read, never
  // pressed.
  //
  // **A gate the city could not report is marked rather than left to be
  // noticed**, because it is not a gate a person can trust; it carries the
  // mark this product uses for anything that needs a person (`asks`,
  // client/Spec.lean §7C), which survives a forced colour mode where a
  // colour alone would not. An open sandbox is the setting the User chose
  // for the building and nothing they must act on, so its words alone
  // state it: the mark beside it read as a fault on a row that stands
  // under every message.
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

  const sandbox = $derived.by((): string => {
    const held = building;
    if (held === undefined || !("building" in held)) return "—";
    const limits = held.building.sandbox;
    if (limits === null || limits === undefined) return say($lang, "facts_sandbox_open");
    return fill(say($lang, "facts_sandbox_mounts"), { n: String(limits.mounts.length) });
  });
</script>

<span class={[FACT, gate.worrying ? "asks" : ""]}>
  <Glyph name="gate" size="sm" />
  <span class="sr-only">{say($lang, "facts_autonomy")}</span>
  {gate.words}
</span>
<span class={FACT}>
  <Glyph name="sandbox" size="sm" />
  <span class="sr-only">{say($lang, "facts_sandbox")}</span>
  {sandbox}
</span>

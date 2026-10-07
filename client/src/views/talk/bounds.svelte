<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // What bounds a run in this room, as read-only facts in the chosen
  // session's gauge (docs/frontend-method.md §7D): who answers the gate,
  // and the sandbox the room's building boxes a run in when one
  // restricts it (`sandbox.svelte`). They are read, never pressed; the
  // gate is chosen in settings, not on Main.
  //
  // **A gate the city could not report is marked rather than left to be
  // noticed**, because it is not a gate a person can trust; it carries the
  // mark this product uses for anything that needs a person (`asks`,
  // client/Spec.lean §7C), which survives a forced colour mode where a
  // colour alone would not.
  //
  // Both facts are drawn in the settings row's one shape for a fact
  // (`fact.look.svelte`); this file decides what they say and writes no
  // class.
  import { readAnswer } from "../../core/answered";
  import { QUERIES } from "../../core/asking";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Address } from "../../wire";
  import Fact from "./fact.look.svelte";
  import Sandbox from "./sandbox.svelte";

  interface Props {
    readonly room: Address | null;
  }

  const { room }: Props = $props();

  const u = ui();
  const { lang } = u;
  const governance = u.conn.asking.ask(QUERIES.governance);

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
</script>

<Fact glyph="gate" heard={say($lang, "facts_autonomy")} asks={gate.worrying}>{gate.words}</Fact>
<Sandbox {room} />

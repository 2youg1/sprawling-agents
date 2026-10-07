<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The seat of the uses (`./usage_uses`): it says every row in the
  // reader's language and on the reader's clock, and draws whatever
  // `./usage_uses.look.svelte` is.
  import { fill, say } from "../../core/lang";
  import { clock } from "../../core/time";
  import { ui } from "../../ui";
  import type { DayCount } from "../../wire";
  import { dayLine, outcomeWord } from "./usage_uses";
  import type { UseRow, UsesLook } from "./usage_uses";
  import Look from "./usage_uses.look.svelte";

  interface Props {
    readonly uses: readonly UseRow[];
    readonly days: readonly DayCount[];
  }

  const { uses, days }: Props = $props();
  const { lang } = ui();

  const look = $derived.by((): UsesLook => {
    const last = uses.at(-1);
    if (last === undefined) return { kind: "never", said: say($lang, "usage_never") };
    return {
      kind: "used",
      count: fill(say($lang, "usage_count"), { n: String(uses.length), when: clock($lang, last.at) }),
      days: dayLine(days),
      fold: say($lang, "usage_every_use"),
      uses: uses.map((used) => ({
        at: clock($lang, used.at),
        who: used.resident ?? used.run,
        part: used.part ?? undefined,
        outcome: say($lang, outcomeWord(used.outcome)),
      })),
    };
  });
</script>

<Look {...look} />

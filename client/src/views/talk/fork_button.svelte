<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- The one branch action, revealed where a hand rests or a focus lands
(ux A7), on a person's words, a reply and a call alike. This is its
seat: it owns the words and the wiring (`fork_button.ts`), and
`fork_button.look.svelte` draws it.

`onHover` reports the entry under the hand, which is the one the
`fork.here` chord branches from; a press branches from this entry. -->
<script lang="ts">
  import { ui } from "../../ui";
  import { say } from "../../core/lang";
  import { forkLookOf } from "./fork_button";
  import Look from "./fork_button.look.svelte";
  import { planFork } from "./forking";
  import type { ForkEntry, ForkPlan } from "./forking";
  import type { RunId } from "../../wire";

  interface Props {
    readonly run: RunId;
    readonly entry: ForkEntry;
    readonly onFork?: ((plan: ForkPlan) => void) | undefined;
    readonly onHover: (entry: ForkEntry | null) => void;
  }

  const { run, entry, onFork, onHover }: Props = $props();

  const { lang } = ui();

  const look = $derived(
    forkLookOf(
      entry,
      { label: say($lang, "fork_here"), hint: say($lang, "fork_here_hint") },
      {
        pick: (picked) => onFork?.(planFork(run, picked)),
        hover: onHover,
      },
    ),
  );
</script>

<Look {...look} />

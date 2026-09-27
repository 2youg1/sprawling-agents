<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- What a person said, drawn as a different kind of thing from what the
model said rather than as the same thing with a different label.

**A person is a shape; the model is the page.** A filled bubble,
right-aligned and held to 83% of the column, reads as one utterance; an
answer with no container at all, running the full measure, reads as a
document.

`entry` is null where the words cannot be branched from, and `onFork` is
absent where the page cannot branch at all; either one hides the fork
action. -->
<script lang="ts">
  import { ui } from "../../ui";
  import { clock } from "../../core/time";
  import ForkButton from "./fork_button.svelte";
  import type { ForkEntry, ForkPlan } from "./forking";
  import type { RunId } from "../../wire";

  interface Props {
    readonly text: string;
    readonly label: string;
    readonly at: number | undefined;
    readonly entry: ForkEntry | null;
    readonly run: RunId;
    readonly onFork?: ((plan: ForkPlan) => void) | undefined;
    readonly onHover: (entry: ForkEntry | null) => void;
  }

  const { text, label, at, entry, run, onFork, onHover }: Props = $props();

  const u = ui();
  const { lang } = u;
</script>

<div class="group relative my-base flex flex-col items-end">
  {#if entry !== null && onFork !== undefined}
    <ForkButton {entry} {run} {onFork} {onHover} />
  {/if}
  <div
    class="max-w-[83%] rounded-panel bg-speech px-pane py-base text-body leading-relaxed whitespace-pre-wrap wrap-anywhere"
  >
    {text}
  </div>
  <div class="mt-tight text-note text-text-faint">
    {label}{#if at !== undefined} · {clock($lang, at)}{/if}
  </div>
</div>

<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- The head of one message: who said it, the frozen facts it was said
under, when, and - once the turn is in the Ledger - how fast it came
(refrain §3-3, client/Spec.lean §4-44). One line of note text, the speaker in
the label weight and everything else faint, so a thread reads as a
column of names with words under them.

**Each figure is drawn only where the screen has no other home for it**
(client/Spec.lean §7D). The model is a fact of the session, so it stands on
the first head and again only where a turn answered with a different
model; the time is to the second, because the tool lines carry the
milliseconds; the cost of a turn is not drawn in the zen and blend tiers
at all (the person's ruling), and the panorama's session sheet is where
it lives. -->
<script lang="ts">
  import { say } from "../../core/lang";
  import { hhmmss } from "../../core/time";
  import { ui } from "../../ui";
  import Sparkline from "./sparkline.svelte";
  import { isoOf, landedWords } from "./timing";

  interface Props {
    readonly who: string;
    readonly at: number;
    // The frozen facts this message was said under - the model, and on a
    // session's first head the mode - where the head is the one that
    // states them; `null` elsewhere.
    readonly model: string | null;
    // Time to first content, once the Ledger holds a measured one.
    readonly ttft: number | null;
    // The rhythm this page watched the reply arrive in, if it watched.
    readonly rhythm: readonly number[] | null;
  }

  const { who, at, model, ttft, rhythm }: Props = $props();

  const { lang } = ui();
</script>

<div class="flex min-w-0 items-baseline gap-base overflow-hidden text-note whitespace-nowrap text-text-faint">
  <span class="shrink-0 font-label text-label text-text">{who}</span>
  {#if model !== null}
    <span class="min-w-0 truncate">{model}</span>
  {/if}
  <time class="figure shrink-0" datetime={isoOf(at)}>{hhmmss(at)}</time>
  {#if ttft !== null}
    <span class="figure shrink-0">{say($lang, "talk_ttft")} {landedWords(ttft)}</span>
  {/if}
  {#if rhythm !== null}
    <Sparkline {rhythm} />
  {/if}
</div>

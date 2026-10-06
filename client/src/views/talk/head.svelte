<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- The head of one message: who said it, the frozen facts it was said
under, when, and - once the turn is in the Ledger - how fast it came:
time to first content, then the output rate (refrain §3-3, client/Spec.lean
§4-44). One line of note text, the speaker in
the label weight and everything else faint, so a thread reads as a
column of names with words under them.

**Each figure is drawn only where the screen has no other home for it**
(docs/frontend-method.md §7D). The model is a fact of the session, so it stands on
the first head and again only where a turn answered with a different
model; the time is to the second, because the tool lines carry the
milliseconds; the cost of a turn is not drawn in the zen and blend tiers
at all (the person's ruling), and the panorama's session sheet is where
it lives. -->
<script lang="ts">
  import { fill, say } from "../../core/lang";
  import { hhmmss, isoInstant } from "../../core/time";
  import { ui } from "../../ui";
  import Sparkline from "./sparkline.svelte";
  import { tookWords } from "./timing";
  import type { Took } from "./timing";

  interface Props {
    readonly who: string;
    readonly at: number;
    // The session facts this message was said under - the model, and on
    // a session's first head its effort, policy and any sandbox that
    // restricts it - where the head is the one that states them; `null`
    // elsewhere.
    readonly model: string | null;
    // Time to first content, once the Ledger holds a measured one.
    readonly ttft: Took | null;
    // Output tokens a second, once the Ledger holds both moments.
    readonly tps: number | null;
    // The rhythm this page watched the reply arrive in, if it watched.
    readonly rhythm: readonly number[] | null;
  }

  const { who, at, model, ttft, tps, rhythm }: Props = $props();

  const { lang } = ui();
</script>

<!-- Inline flow rather than one clipped row: the first head carries the
session's frozen facts, which can outrun the column, and a fact cut off
by an ellipsis is a fact the head no longer states. A space and the
tight margin make the gap the row had; short figures keep their words
together, and the facts wrap where the column ends. -->
<div class="min-w-0 text-note text-text-faint">
  <span class="me-tight font-label text-label whitespace-nowrap text-text">{who}</span>
  {#if model !== null}
    <span class="me-tight wrap-anywhere">{model}</span>
  {/if}
  <time class="figure me-tight whitespace-nowrap" datetime={isoInstant(at)}>{hhmmss(at)}</time>
  {#if ttft !== null}
    <span class="figure me-tight whitespace-nowrap">{say($lang, "talk_ttft")} {tookWords(ttft, $lang)}</span>
  {/if}
  {#if tps !== null}
    <span class="figure me-tight whitespace-nowrap">{fill(say($lang, "talk_tps"), { n: String(Math.round(tps)) })}</span>
  {/if}
  {#if rhythm !== null}
    <Sparkline {rhythm} />
  {/if}
</div>

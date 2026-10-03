<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- What a resident said to this room, drawn as a letter rather than
as the User's bubble (client D86): left-aligned, unfilled, with an
accent edge, and headed by who sent it, linked to the sender's room,
and when it arrived. The wire does not carry the signal's kind or the
sending run, so the head says "letter" and links the room. -->
<script lang="ts">
  import { ui } from "../../ui";
  import { say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { clock } from "../../core/time";
  import { called, residentAt } from "./naming";

  interface Props {
    readonly from: string | null | undefined;
    readonly said: string;
    readonly t: number;
  }

  const { from, said, t }: Props = $props();

  const u = ui();
  const { lang } = u;
  const sender = $derived(residentAt(from));
</script>

<article
  class="my-base mr-auto max-w-[83%] rounded-card border border-edge-panel border-l-2 border-l-accent px-pane py-base"
  aria-label={say($lang, "talk_letter")}
>
  <div class="mb-tight flex items-baseline gap-snug text-note text-text-faint">
    <span>{say($lang, "talk_letter")}</span>
    {#if sender !== null}
      <a href={toFragment({ kind: "talk", address: sender })} class="text-text-quiet hover:text-text">{called(sender, null, $lang)}</a>
    {:else if from !== undefined && from !== null}
      <span class="text-text-quiet">{from}</span>
    {/if}
    <span>· {clock($lang, t)}</span>
  </div>
  <div class="text-body leading-relaxed whitespace-pre-wrap wrap-anywhere">{said}</div>
</article>

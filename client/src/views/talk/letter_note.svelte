<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- What a resident said to this room, drawn as a letter rather than
as the User's bubble (client D86): left-aligned, unfilled, with an
accent edge, and headed by the kind it was sent as, who sent it, linked
to the sender's room, the session that sent it, and when it arrived
(wire D43, client D89). A letter whose sending was not paired says
"letter" and links the room only, rather than guess either. -->
<script lang="ts">
  import { ui } from "../../ui";
  import { say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { clock } from "../../core/time";
  import type { RunId } from "../../wire";
  import { kindSaid } from "./inbox";
  import { called, residentAt } from "./naming";

  interface Props {
    readonly from: string | null | undefined;
    readonly said: string;
    readonly kind: string | null;
    readonly session: RunId | null;
    readonly t: number;
  }

  const { from, said, kind, session, t }: Props = $props();

  const u = ui();
  const { lang } = u;
  const sender = $derived(residentAt(from));
  const head = $derived(kind === null ? say($lang, "talk_letter") : kindSaid($lang, kind));
</script>

<article
  class="my-base mr-auto max-w-[83%] rounded-card border border-edge-panel border-l-2 border-l-accent px-pane py-base"
  aria-label={say($lang, "talk_letter")}
>
  <div class="mb-tight flex flex-wrap items-baseline gap-snug text-note text-text-faint">
    <span>{head}</span>
    {#if sender !== null}
      <a href={toFragment({ kind: "talk", address: sender })} class="text-text-quiet hover:text-text">{called(sender, null, $lang)}</a>
    {:else if from !== undefined && from !== null}
      <span class="text-text-quiet">{from}</span>
    {/if}
    {#if session !== null}
      <a href={toFragment({ kind: "run", run: session })} class="text-text-quiet hover:text-text">{say($lang, "talk_letter_session")}</a>
    {/if}
    <span>· {clock($lang, t)}</span>
  </div>
  <div class="text-body leading-relaxed whitespace-pre-wrap wrap-anywhere">{said}</div>
</article>

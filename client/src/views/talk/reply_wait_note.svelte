<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- A run waiting for another room's reply (client D86): the room
waited on, linked, and while the wait is open the time left before its
deadline on the page's one clock; once it ends, which of the three
endings it was, each in its own ink. The reply itself is an arrival of
its own, drawn as a letter. -->
<script lang="ts">
  import { ui } from "../../ui";
  import { fill, say } from "../../core/lang";
  import type { Key } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { lasted } from "../../core/time";
  import { called } from "./naming";
  import { ticker } from "./timing";
  import type { Address, ReplyEnd, ReplyEnded } from "../../wire";

  interface Props {
    readonly on: Address;
    readonly until: number;
    readonly ended: ReplyEnded | null | undefined;
  }

  const { on, until, ended }: Props = $props();

  const u = ui();
  const { lang } = u;
  const clock = ticker(u.now);

  const ENDING: Readonly<Record<ReplyEnd, { readonly key: Key; readonly ink: string }>> = {
    reply: { key: "talk_reply_ended_reply", ink: "text-accent" },
    timeout: { key: "talk_reply_ended_timeout", ink: "text-alert" },
    left: { key: "talk_reply_ended_left", ink: "text-text-faint" },
  };

  const left = $derived(until - $clock);
</script>

<div class="my-snug flex flex-wrap items-baseline gap-snug text-note text-text-faint">
  {#if ended === undefined || ended === null}
    <span>{say($lang, "talk_reply_wait")}</span>
    <a href={toFragment({ kind: "talk", address: on })} class="text-text-quiet hover:text-text">{called(on, null, $lang)}</a>
    <span class={left > 0 ? "text-text-quiet" : "text-alert"}>
      · {left > 0 ? fill(say($lang, "talk_reply_left"), { left: lasted(left) }) : say($lang, "talk_reply_due")}
    </span>
  {:else}
    <span>{say($lang, "talk_reply_waited")}</span>
    <a href={toFragment({ kind: "talk", address: on })} class="text-text-quiet hover:text-text">{called(on, null, $lang)}</a>
    <span class={ENDING[ended.by].ink}>· {say($lang, ENDING[ended.by].key)}</span>
  {/if}
</div>

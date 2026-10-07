<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts">
  // The signals queued for this room and not yet read by a run, asked
  // without taking them (`Query::InboxView`). The city bar counts them
  // across the city; this is where a person sees which they are.
  // Each row says the kind, the first line of the text and who sent it,
  // and leads to the sender's room (client D86).
  import { say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { ago } from "../../core/time";
  import Look from "./inbox.look.svelte";
  import { kindSaid, waitingSaid } from "./inbox";
  import type { InboxRow } from "./inbox";
  import { called, residentAt } from "./naming";
  import NotePlace from "./note_place.svelte";
  import { ui } from "../../ui";
  import type { Address } from "../../wire";

  interface Props {
    readonly addr: Address;
  }

  const { addr }: Props = $props();
  const u = ui();
  const { lang } = u;

  const answer = $derived(u.conn.asking.ask({ inbox_view: { addr } }));
  const rows = $derived.by((): InboxRow[] => {
    const held = $answer;
    const waiting = held !== undefined && "inbox" in held ? held.inbox.waiting : [];
    return waiting.map((line) => {
      const sender = residentAt(line.from);
      return {
        key: line.id,
        href: sender === null ? undefined : toFragment({ kind: "talk", address: sender }),
        kind: kindSaid($lang, line.kind),
        first: line.first_line ?? "",
        from: sender === null ? line.from : called(sender, null, $lang),
        ago: ago($lang, line.at, u.now()),
      };
    });
  });
</script>

{#if rows.length > 0}
  <NotePlace rhythm="shape">
    <Look label={say($lang, "inbox_waiting")} heading={waitingSaid($lang, rows.length)} {rows} />
  </NotePlace>
{/if}

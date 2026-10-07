<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // Who is listening in this room (docs/frontend-method.md §7I), read in the room
  // chip's menu above the rooms it offers: the city, the building, the
  // resident and the run, one row each, and beside each the segment of
  // the prompt that the room's newest run was told for it - its size,
  // the files it was read from, and what the budget cut off them.
  //
  // It is the run page's prompt lens (`run/prompt.svelte`) cut to what a
  // person checks before writing - who hears this and from which files -
  // and it reads the same answer (`Query::Prefix`). The text itself stays
  // on the run page: a menu over the box is no place to read four
  // documents, and it holds no control of its own, because the menu's
  // keys belong to its list.
  //
  // This file is the seat (client D95): it asks the city, `listening.ts`
  // reads the answer into words, and `listening.look.svelte` draws them.
  import { ui } from "../../ui";
  import type { Address, PrefixSegment } from "../../wire";
  import { sessionRunOf } from "./gauge";
  import { listeningOf } from "./listening";
  import Look from "./listening.look.svelte";

  interface Props {
    readonly room: Address;
  }

  const { room }: Props = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;

  const run = $derived(sessionRunOf($belief, room));
  let segments = $state<readonly PrefixSegment[]>([]);
  $effect(() => {
    const at = run;
    segments = [];
    if (at === undefined) return;
    return u.conn.asking.ask({ prefix: { run: at } }).subscribe((held) => {
      segments = held !== undefined && "prefix" in held ? held.prefix.segments : [];
    });
  });

  const look = $derived(listeningOf($lang, { city: $belief.city, room, segments, told: run !== undefined }));
</script>

<Look {...look} />

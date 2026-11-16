<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
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

  const look = $derived(
    listeningOf($lang, { city: $belief.city, room, segments, told: run !== undefined }),
  );
</script>

<Look {...look} />

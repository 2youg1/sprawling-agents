<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import { SvelteSet } from "svelte/reactivity";

  import { buildingOf } from "../../core/route";
  import { ui } from "../../ui";
  import type { RunId } from "../../wire";
  import { lookOf } from "./prompt";
  import type { Sent } from "./prompt";
  import Look from "./prompt.look.svelte";

  interface Props {
    readonly run: RunId;
  }

  const { run }: Props = $props();

  const u = ui();
  const lang = u.lang;

  const open = new SvelteSet<string>();
  let receipt = $state<string | null>(null);

  const asked = $derived(u.conn.asking.ask({ prefix: { run } }));
  const sent = $derived.by((): Sent | undefined => {
    const held = $asked;
    if (held === undefined) return undefined;
    return "prefix" in held ? held.prefix : { segments: [], tools: null };
  });

  $effect(() => {
    const at = receipt;
    if (at === null) return;
    const timer = setTimeout(() => {
      if (receipt === at) receipt = null;
    }, 1200);
    return () => {
      clearTimeout(timer);
    };
  });

  const look = $derived(
    lookOf(sent, { open, receipt }, $lang, {
      toggle: (hash) => {
        if (!open.delete(hash)) open.add(hash);
      },
      copy: (segment) => {
        void navigator.clipboard.writeText(segment.text);
        receipt = segment.hash;
      },
      visit: (source) => {
        u.go({ kind: "building", address: buildingOf(source) });
      },
    }),
  );
</script>

<Look {...look} />

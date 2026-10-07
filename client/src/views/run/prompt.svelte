<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The seat of the prompt lens: what this run was told, in the four
  // segments it was sent as. It asks the city, holds which segments are
  // open and whose copy receipt shows, and draws whatever
  // `./prompt.look.svelte` is (`./prompt` decides the rest).
  //
  // The bytes come from the store, addressed by the hash the ledger
  // recorded; a segment the store no longer holds says so rather than
  // showing an empty box a reader would take for an empty prompt.

  import { SvelteSet } from "svelte/reactivity";

  import { buildingOf } from "../../core/route";
  import { ui } from "../../ui";
  import type { PrefixSegment, RunId } from "../../wire";
  import { lookOf } from "./prompt";
  import Look from "./prompt.look.svelte";

  interface Props {
    readonly run: RunId;
  }

  const { run }: Props = $props();

  const u = ui();
  const lang = u.lang;

  // Each segment folds on its own: a person compares two segments and
  // should not lose the first while opening the second.
  const open = new SvelteSet<string>();
  // Which segment's copy receipt is showing right now (ux A7).
  let receipt = $state<string | null>(null);

  const asked = $derived(u.conn.asking.ask({ prefix: { run } }));
  const segments = $derived.by((): readonly PrefixSegment[] | undefined => {
    const held = $asked;
    if (held === undefined) return undefined;
    return "prefix" in held ? held.prefix.segments : [];
  });

  // A copy receipt lasts long enough to be seen and no longer (ux A7).
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
    lookOf(segments, { open, receipt }, $lang, {
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

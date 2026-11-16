<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  import type { SettledSearch } from "../../../wire";

  export interface SearchCardProps {
    readonly settled?: SettledSearch | undefined;
  }
</script>

<script lang="ts">
  import { ui } from "../../../ui";
  import { HALL } from "../../shared/buildings";
  import { lookOf } from "./search";
  import Look from "./search.look.svelte";
  import { freshSearch, refuse, settle } from "./search_editor";
  import type { SearchEditor, SearchHands } from "./search_editor";

  const { settled }: SearchCardProps = $props();
  const u = ui();
  const lang = u.lang;
  const belief = u.conn.belief;
  const config = u.conn.asking.ask({ config: { addr: HALL } });

  const answer = $derived.by((): SettledSearch | undefined => {
    if (settled !== undefined) return settled;
    const held = $config;
    return held !== undefined && "config" in held ? held.config.search : undefined;
  });

  const editor: SearchEditor = $state(freshSearch());

  $effect(() => {
    if (answer !== undefined) settle(editor, answer);
  });
  $effect(() => {
    const refused = $belief.refusal;
    if (refused === null || editor.sent === null) return;
    refuse(editor, refused);
    u.conn.dismissRefusal();
  });

  function hands(held: SettledSearch): SearchHands {
    return { settled: () => held, lang: () => $lang, send: (command) => u.send(command) };
  }

  const look = $derived(answer === undefined ? undefined : lookOf(editor, hands(answer)));
</script>

{#if look !== undefined}
  <Look {...look} />
{/if}

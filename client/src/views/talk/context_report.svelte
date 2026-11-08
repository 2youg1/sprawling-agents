<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The context report for one run: asks Prefix and PromptShape, joins
  // them with the session's fullness, and hands a look to the drawing.
  // Close is the inspector tab's own control — same door as a document.
  import { QUERIES } from "../../core/asking";
  import { ui } from "../../ui";
  import type { Address, PrefixAnswer, PromptShapeAnswer, RoundsAnswer, RunId } from "../../wire";
  import Look from "./context_report.look.svelte";
  import { reportOf } from "./context_report";
  import { contextOf, remindersOf } from "./gauge";
  import type { Reminders } from "./gauge";

  interface Props {
    readonly run: RunId;
    readonly room: Address | null;
  }

  const { run, room }: Props = $props();
  const u = ui();
  const lang = u.lang;
  const endpoints = u.conn.asking.ask(QUERIES.endpoints);

  let prefix = $state<PrefixAnswer | null>(null);
  let shape = $state<PromptShapeAnswer | null>(null);
  let rounds = $state<RoundsAnswer | undefined>(undefined);
  let reminders = $state<Reminders>(remindersOf(undefined));

  $effect(() => {
    prefix = null;
    return u.conn.asking.ask({ prefix: { run } }).subscribe((held) => {
      prefix = held !== undefined && "prefix" in held ? held.prefix : null;
    });
  });

  $effect(() => {
    shape = null;
    return u.conn.asking.ask({ prompt_shape: { run } }).subscribe((held) => {
      shape = held !== undefined && "prompt_shape" in held ? held.prompt_shape : null;
    });
  });

  $effect(() => {
    rounds = undefined;
    return u.conn.asking.ask({ rounds: { run } }).subscribe((held) => {
      rounds = held !== undefined && "rounds" in held ? held.rounds : undefined;
    });
  });

  $effect(() => {
    const at = room;
    reminders = remindersOf(undefined);
    if (at === null) return;
    return u.conn.asking.ask({ config: { addr: at } }).subscribe((held) => {
      reminders = remindersOf(held);
    });
  });

  const look = $derived(
    reportOf(
      $lang,
      run,
      prefix,
      shape,
      contextOf(
        rounds?.turns ?? [],
        $endpoints !== undefined && "endpoints" in $endpoints ? $endpoints.endpoints.endpoints : [],
        reminders,
      ),
    ),
  );
</script>

<Look {look} />

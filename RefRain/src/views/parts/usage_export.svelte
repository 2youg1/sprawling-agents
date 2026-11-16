<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  
  
  import { onDestroy } from "svelte";
  import { get } from "svelte/store";

  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { ExportFormat, Query, UsageKind } from "../../wire";
  import { saveFile } from "../editor/saved_file";
  import { exportedOf, FORMATS, TYPES } from "./usage_export";
  import type { Exported } from "./usage_export";
  import Look from "./usage_export.look.svelte";

  interface Props {
    readonly what: UsageKind;
  }

  const { what }: Props = $props();
  const u = ui();
  const lang = u.lang;
  let waiting = $state<ExportFormat | null>(null);
  let missed = $state<Exported | null>(null);
  
  const waits: Record<ExportFormat, (() => void) | undefined> = {
    jsonl: undefined,
    csv: undefined,
  };
  onDestroy(() => {
    for (const format of FORMATS) waits[format]?.();
  });

  function press(format: ExportFormat): void {
    const query: Query = { usage_export: { what, format } };
    waiting = format;
    missed = null;
    const held = u.conn.asking.ask(query);
    
    
    const before = get(held);
    u.conn.asking.refresh(query);
    const off = held.subscribe((answer) => {
      if (answer === undefined || answer === before) return;
      const exported = exportedOf(answer);
      if (exported === undefined) return;
      // wording-ok: a file name, spelled the same in every language.
      if (exported.kind === "file")
        saveFile(`${what}-usage.${format}`, TYPES[format], exported.body);
      else missed = exported;
      waiting = null;
      waits[format] = undefined;
      queueMicrotask(off);
    });
    waits[format] = off;
  }

  const unavailable = $derived.by(() => {
    if (missed?.kind !== "unavailable") return undefined;
    return {
      said: fill(say($lang, "answer_unavailable"), { query: missed.query }),
      reason: missed.reason,
    };
  });
</script>

<Look
  buttons={FORMATS.map((format) => ({
    format,
    label: say($lang, format === "jsonl" ? "usage_export_jsonl" : "usage_export_csv"),
    loading: waiting === format,
    press: () => {
      press(format);
    },
  }))}
  {unavailable}
/>

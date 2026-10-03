<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The two export buttons of a usage panel. The city writes the rows
  // (`Query::UsageExport`, wire D33) so the columns have one author; the
  // page asks again on every press, because a held answer would hand the
  // User the uses as they stood when the panel first opened, and gives
  // the text to the browser's own download. Nothing is written in the
  // city's folder.
  import type { ExportFormat, UsageKind } from "../../wire";

  const TYPES: Record<ExportFormat, string> = {
    jsonl: "application/x-ndjson",
    csv: "text/csv",
  };
</script>

<script lang="ts">
  import { get } from "svelte/store";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Query } from "../../wire";
  import Button from "./button.svelte";
  import { saveFile } from "../refrain/saved_file";

  interface Props {
    readonly what: UsageKind;
  }

  const { what }: Props = $props();
  const u = ui();
  const lang = u.lang;
  let waiting = $state<ExportFormat | null>(null);

  function press(format: ExportFormat): void {
    const query: Query = { usage_export: { what, format } };
    waiting = format;
    const held = u.conn.asking.ask(query);
    // The answer a press before this one left: the press is answered by
    // the next one to arrive, never by that.
    const before = get(held);
    u.conn.asking.refresh(query);
    const off = held.subscribe((answer) => {
      if (answer === undefined || answer === before || !("usage_export" in answer)) return;
      // wording-ok: a file name, spelled the same in every language.
      saveFile(`${what}-usage.${format}`, TYPES[format], answer.usage_export.body);
      waiting = null;
      queueMicrotask(off);
    });
  }
</script>

<div class="flex items-center gap-snug">
  <Button label={say($lang, "usage_export_jsonl")} tone="quiet" loading={waiting === "jsonl"} onPress={() => { press("jsonl"); }} />
  <Button label={say($lang, "usage_export_csv")} tone="quiet" loading={waiting === "csv"} onPress={() => { press("csv"); }} />
</div>

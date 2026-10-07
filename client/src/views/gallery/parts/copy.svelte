<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The copy key in both forms and all three of its states. The seat
  // holds its state until a press, so the receipt and the refusal are
  // drawn by handing the look the value the seat would hand it after
  // that press; the first row is the seat itself.
  import { fill, say } from "../../../core/lang";
  import { ui } from "../../../ui";
  import { lookOf, REST } from "../../parts/copy";
  import type { Copied } from "../../parts/copy";
  import Copy from "../../parts/copy.svelte";
  import CopyLook from "../../parts/copy.look.svelte";
  import Button from "../../parts/button.svelte";
  import UsageExportLook from "../../parts/usage_export.look.svelte";
  import Case from "../case.svelte";

  const { lang } = ui();

  const COMMAND = "cargo install sprawling --locked";
  const STATES: readonly Copied[] = [
    REST,
    { kind: "copied" },
    { kind: "refused", why: { kind: "refused", said: "Document is not focused." } },
    { kind: "refused", why: { kind: "absent" } },
  ];
  const idle = (): void => undefined;
  // The city's reason, in its own English, exactly as the wire carries it.
  const UNREAD = "the usage index is still being built";
</script>

<Case label="copy · beside a command, at rest, copied, refused, and with no clipboard">
  <div class="flex flex-col gap-snug">
    <div class="flex min-w-0 items-center gap-snug">
      <code class="min-w-0 truncate font-mono text-note text-text">{COMMAND}</code>
      <Button label={say($lang, "link_retry")} tone="quiet" />
      <Copy text={COMMAND} />
    </div>
    {#each STATES as copied, at (at)}
      <div class="flex min-w-0 items-center gap-snug">
        <CopyLook {...lookOf({ text: COMMAND }, copied, $lang, idle)} />
        <CopyLook {...lookOf({ text: COMMAND, note: say($lang, "code_copy_note"), form: "bare" }, copied, $lang, idle)} />
      </div>
    {/each}
  </div>
</Case>

<Case label="usage export · the city could not answer the last press">
  <UsageExportLook
    buttons={[
      { format: "jsonl", label: say($lang, "usage_export_jsonl"), loading: false, press: idle },
      { format: "csv", label: say($lang, "usage_export_csv"), loading: false, press: idle },
    ]}
    unavailable={{ said: fill(say($lang, "answer_unavailable"), { query: "usage_export" }), reason: UNREAD }}
  />
</Case>

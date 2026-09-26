<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // Every shared control, in each state it can be in. They are gathered
  // in one component rather than spread through the index because the
  // states that need a signal - a chosen model, a ticked row, an open
  // dialog - are held here beside the fixtures that show them.

  import type { PopoverColumn } from "../parts/popover";
  import type { ModelRow } from "./served";

  // One refusal as the wire carries it: the action that failed, its
  // stable code, what it was against, and what a person can do next.
  // The words are the city's own English originals, which the notice
  // folds under its translated title - fixture data exactly as they
  // arrive.
  interface Refusal {
    readonly action: string;
    readonly code: string;
    readonly subject: string;
    readonly recovery: string;
  }

  interface Entry {
    // Already formatted by the fixture's clock.
    readonly at: string;
    readonly count: number;
    readonly one: Refusal;
  }

  function refusal(code: string, subject: string, recovery: string): Refusal {
    return { action: "attach an endpoint", code, subject, recovery };
  }

  function entry(at: string, count: number, one: Refusal): Entry {
    return { at, count, one };
  }

  // Three refusals across a quarter hour, arriving twelve times: the
  // drawer keeps every arrival as its own row with the count that
  // merged into it.
  const MISSING_KEY = refusal("E_CREDENTIAL_MISSING", "zenmux", "file a key for this provider, then attach it again");
  const REFUSED_LINK = refusal("E_LINK_REFUSED", "local", "start the endpoint, then try this again");
  const STALE_DRAFT = refusal("E_DRAFT_STALE", "config.toml", "reload the file, then make the change again");

  const DRAWER: readonly Entry[] = [
    entry("03:35", 3, MISSING_KEY),
    entry("03:34", 1, REFUSED_LINK),
    entry("03:33", 1, STALE_DRAFT),
    entry("03:31", 2, MISSING_KEY),
    entry("03:30", 1, REFUSED_LINK),
    entry("03:28", 1, STALE_DRAFT),
    entry("03:27", 1, MISSING_KEY),
    entry("03:26", 1, REFUSED_LINK),
    entry("03:25", 1, STALE_DRAFT),
    entry("03:24", 1, MISSING_KEY),
    entry("03:23", 1, REFUSED_LINK),
    entry("03:22", 1, STALE_DRAFT),
  ];
</script>

<script lang="ts">
  import { fill, say } from "../../core/lang";
  import { referenceFor, referenceText } from "../../core/enrol";
  import { ui } from "../../ui";
  import Button from "../parts/button.svelte";
  import Combobox from "../parts/combobox.svelte";
  import Dialog from "../parts/dialog.svelte";
  import EmptyState from "../parts/empty.svelte";
  import Field from "../parts/field.svelte";
  import Notice from "../parts/notice.svelte";
  import Popover from "../parts/popover.svelte";
  import { Table } from "../parts/table";
  import type { Column } from "../parts/table";
  import Tip from "../parts/tip.svelte";
  import Case from "./case.svelte";
  import { CHOSEN, MODELS } from "./served";

  const { lang } = ui();

  let host = $state("api.zenmux.ai");
  let bad = $state("api_gateway.internal");
  let key = $state(referenceText(referenceFor("zenmux")));
  let measured = $state("a million");
  let model = $state<string | null>(null);
  const modelChoices = MODELS.map((each) => ({ value: each.id, label: each.id, note: each.context }));
  const pickModel = (value: string): void => { model = value; };
  let rows = $state<readonly ModelRow[]>(MODELS);
  let picked = $state<readonly string[]>([CHOSEN.id]);
  let asking = $state(false);

  const columns: readonly Column<ModelRow>[] = [
    {
      key: "id",
      header: say($lang, "part_model_id"),
      render: idCell,
      compare: (a, b) => a.id.localeCompare(b.id),
    },
    {
      key: "context",
      header: say($lang, "part_context_window"),
      render: contextCell,
      compare: (a, b) => Number(a.context) - Number(b.context),
    },
    {
      key: "ceiling",
      header: say($lang, "part_output_ceiling"),
      render: ceilingCell,
      editable: {
        text: (row) => row.ceiling,
        onEdit: (row, text) => {
          rows = rows.map((each) =>
            each.id === row.id ? { ...each, ceiling: text } : each,
          );
        },
      },
    },
  ];

  // The point picker's two columns: where a run turned, and what it did
  // in that turn. Turn numbers are filled from the one phrase that
  // numbers a turn; the call rows are the wire's own verb and subject.
  function forkColumns(): readonly PopoverColumn[] {
    return [
      {
        id: "turns",
        label: "run_turns",
        rows: [
          {
            id: "turn-3",
            label: fill(say($lang, "run_turn_n"), { n: "3" }),
            secondary: "03:35",
            chosen: true,
          },
          { id: "turn-2", label: fill(say($lang, "run_turn_n"), { n: "2" }), secondary: "03:12" },
          { id: "turn-1", label: fill(say($lang, "run_turn_n"), { n: "1" }), secondary: "02:58" },
        ],
      },
      {
        id: "calls",
        label: "talk_calls",
        rows: [
          { id: "call-11", label: "exec: cargo test -p kernel", chosen: true },
          { id: "call-10", label: "edit: crates/kernel/src/gate/door.rs" },
          { id: "call-4", label: "search: GateOutcome" },
        ],
      },
    ];
  }
</script>

{#snippet idCell(row: ModelRow)}
  <span class="font-mono text-note">{row.id}</span>
{/snippet}

{#snippet contextCell(row: ModelRow)}
  <span class="font-mono text-note">{row.context}</span>
{/snippet}

{#snippet ceilingCell(row: ModelRow)}
  <span class="font-mono text-note">{row.ceiling}</span>
{/snippet}

<Case label="button · four tones">
  <div class="flex flex-wrap items-center gap-snug">
    <Button label={say($lang, "part_save")} tone="primary" />
    <Button label={say($lang, "part_cancel")} tone="secondary" />
    <Button label={say($lang, "dismiss")} tone="quiet" />
    <Button label={say($lang, "part_delete")} tone="destructive" />
  </div>
</Case>

<Case label="button · loading and refused">
  <div class="flex flex-wrap items-center gap-snug">
    <Button label={say($lang, "part_save")} tone="primary" loading />
    <Button label={say($lang, "part_save")} tone="primary" why={say($lang, "part_why_halted")} />
  </div>
</Case>

<!-- Two red edges that mean two things: `error` is the city's answer
and arrives red at once; `:user-invalid` is the browser reading
`pattern`, and turns red only after the person has left the box. -->
<Case label="field · help, the city's error, the browser's user-invalid, mono with fixed parts">
  <div class="flex flex-col gap-base">
    <Field
      label={say($lang, "setup_base_url")}
      help={say($lang, "part_help_base_url")}
      value={host}
      onInput={(value) => {
        host = value;
      }}
      prefix="https://"
      suffix="/v1"
    />
    <Field
      label={say($lang, "setup_base_url")}
      error={say($lang, "part_error_host")}
      value={bad}
      onInput={(value) => {
        bad = value;
      }}
    />
    <Field
      label={say($lang, "part_context_window")}
      help={say($lang, "part_context_window")}
      value={measured}
      onInput={(value) => {
        measured = value;
      }}
      pattern="[0-9]+"
    />
    <Field
      label={say($lang, "setup_key")}
      value={key}
      onInput={(value) => {
        key = value;
      }}
      mono
    />
  </div>
</Case>

<!-- Stacked as the settings page stacks them: the open list covers the second trigger.
The list is positioned and takes no room, so `pb-output` (the list's own max height) grows the section to hold it. -->
<Case label="combobox · open on click, nothing chosen">
  <div class="flex flex-col gap-base pb-output">
    <Combobox label={say($lang, "setup_main")} placeholder={say($lang, "part_search")} empty={say($lang, "part_no_match")}
      choices={modelChoices} value={model} onPick={pickModel} starts="open" />
    <Combobox label={say($lang, "setup_digest")} placeholder={say($lang, "part_search")} empty={say($lang, "part_no_match")}
      choices={modelChoices} value={model} onPick={pickModel} />
  </div>
</Case>

<Case label="table · partial selection, sortable, corrected in place">
  <Table
    caption={say($lang, "setup_models")}
    {columns}
    {rows}
    keyOf={(row: ModelRow) => row.id}
    selection={{
      picked: (row: ModelRow) => picked.includes(row.id),
      onPick: (row: ModelRow, on: boolean) => {
        picked = on ? [...picked, row.id] : picked.filter((each) => each !== row.id);
      },
      allLabel: say($lang, "part_select_all"),
      allPicked: () => picked.length === rows.length,
      onPickAll: (on: boolean) => {
        picked = on ? rows.map((each) => each.id) : [];
      },
    }}
  />
</Case>

<Case label="table · empty">
  <Table
    caption={say($lang, "setup_models")}
    {columns}
    rows={[]}
    keyOf={(row: ModelRow) => row.id}
  >
    {#snippet empty()}
      <EmptyState missing="setup_no_models">
        {#snippet action()}
          <Button label={say($lang, "setup_look")} tone="primary" />
        {/snippet}
      </EmptyState>
    {/snippet}
  </Table>
</Case>

<Case label="dialog · confirming what cannot be undone">
  <Button
    label={say($lang, "setup_remove")}
    tone="destructive"
    onPress={() => {
      asking = true;
    }}
  />
  <Dialog
    open={asking}
    title={say($lang, "part_remove_endpoint")}
    detail={say($lang, "part_remove_detail")}
    confirmLabel={say($lang, "part_delete")}
    cancelLabel={say($lang, "part_cancel")}
    destructive
    onConfirm={() => {
      asking = false;
    }}
    onCancel={() => {
      asking = false;
    }}
  />
</Case>

<!-- The two columns the point picker opens over the composer: the
list is drawn open here because the state worth looking at is the
state it opens into. -->
<Case label="popover · two columns of one choice">
  <div class="pt-palette">
    <div class="pt-output">
      <div class="relative">
        <Popover
          label="fork_pick_title"
          columns={forkColumns()}
          onApply={() => undefined}
          onClose={() => undefined}
        />
      </div>
    </div>
  </div>
</Case>

<!-- The relation `tip.tsx` documents on a control that is named by
its own text: the hint is `aria-describedby`, never the name.
`button.svelte` composes the same relation for `why`; this is the
pattern as a caller outside that component writes it. -->
<Case label="tip · a hint on a control named by its own text">
  <Tip text={say($lang, "part_why_halted")}>
    {#snippet children(hint: string)}
      <button
        type="button"
        class="rounded-control px-base py-tight text-label text-text-quiet hover:bg-raised"
        aria-describedby={hint}
      >
        {say($lang, "part_save")}
      </button>
    {/snippet}
  </Tip>
</Case>

<Case label="notice · inline beneath what it is about">
  <Notice
    seat="inline"
    weight="alert"
    action={MISSING_KEY.action}
    code={MISSING_KEY.code}
    subject={MISSING_KEY.subject}
    recovery={MISSING_KEY.recovery}
    at="03:35"
    count={3}
  />
</Case>

<!-- Two seats, two widths, each shown where it lands. A toast is
capped and floats at the corner; an entry fills the panel it belongs
to. Drawn stacked in one case they read as a page whose right edge
disagrees with itself. -->
<Case label="notices · toasts">
  <div class="flex min-h-output w-full items-end justify-end rounded-card border border-dashed border-edge-input p-snug">
    <div class="flex min-w-0 max-w-measure flex-col gap-tight">
      <Notice
        seat="toast"
        action={MISSING_KEY.action}
        code={MISSING_KEY.code}
        subject={MISSING_KEY.subject}
        recovery={MISSING_KEY.recovery}
        at="03:35"
        count={3}
      >
        {#snippet actions()}
          <Button label={say($lang, "part_undo")} tone="quiet" />
          <Button label={say($lang, "dismiss")} tone="quiet" />
        {/snippet}
      </Notice>
      <Notice
        seat="toast"
        weight="alert"
        action={REFUSED_LINK.action}
        code={REFUSED_LINK.code}
        subject={REFUSED_LINK.subject}
        recovery={REFUSED_LINK.recovery}
        at="03:34"
      />
    </div>
  </div>
</Case>

<Case label="notices · drawer with 12 entries" width={520}>
  <div class="max-h-output min-w-0 overflow-y-auto rounded-card border border-edge-panel">
    {#each DRAWER as entry (entry.at)}
      <Notice
        seat="drawer"
        weight="alert"
        action={entry.one.action}
        code={entry.one.code}
        subject={entry.one.subject}
        recovery={entry.one.recovery}
        at={entry.at}
        count={entry.count}
      />
    {/each}
  </div>
</Case>

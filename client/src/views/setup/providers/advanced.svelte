<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // Everything a person may state about an endpoint and almost nobody
  // has to: what this provider is called, and the five settings a
  // working endpoint never needs changed. Folded away until somebody
  // asks for it, because the form's three boxes are what nearly
  // everybody came to fill in.
  //
  // The two naming boxes lead, because they are the two the form
  // stopped asking for on the way in; each shows its derived value as
  // the placeholder, so an empty box is a statement of what will be
  // used rather than a question nobody answered. The two boxed tables
  // are one snippet used twice: a header every request carries, and a
  // JSON pointer into the body it sends.
  //
  // Every change leaves through `setDraft`, the form's own store
  // setter: the draft is the form's state and this file is a set of
  // boxes standing at a distance from it, which is the same seam the
  // Solid original crossed with its `SetStoreFunction`.

  import type { Key } from "../../../core/lang";
  import type { Draft, Line } from "./draft";
import type { Proxying } from "../../../wire";

  export type SetDraft = <K extends keyof Draft>(part: K, value: Draft[K]) => void;

  export interface AdvancedProps {
    readonly draft: Draft;
    readonly setDraft: SetDraft;
    // A rename retires the last report: what it said was about the
    // name the form carried then.
    readonly onRenamed: () => void;
  }

  // One boxed key-value table: the word over it, the two column
  // words, and the rows it edits through `change`.
  export interface PairTableProps {
    readonly title: string;
    readonly name: string;
    readonly value: string;
    readonly rows: readonly Line[];
    readonly change: (rows: readonly Line[]) => void;
  }

  // The three tuning figures, each with the word it is offered under
  // and the step one arrow key moves it by. A table rather than three
  // boxes: the row below draws itself from it.
  type TuningField = "timeoutMs" | "requestRetries" | "streamIdleMs";

  const TUNING: readonly (readonly [Key, TuningField, number])[] = [
    ["setup_timeout_ms", "timeoutMs", 1000],
    ["setup_request_retries", "requestRetries", 1],
    ["setup_stream_idle", "streamIdleMs", 1000],
  ];
</script>

<script lang="ts">
  import { say } from "../../../core/lang";
  import { ui } from "../../../ui";
  import Button from "../../parts/button.svelte";
  import Field from "../../parts/field.svelte";
  import Segmented from "../../parts/segmented.svelte";
  import { ID_SHAPE, PROXYINGS, blankLine, boxed, idOf, naming, proxyingNote } from "./draft";

  const { draft, setDraft, onRenamed }: AdvancedProps = $props();

  const { lang } = ui();
  const ruleNote = $derived(proxyingNote(draft.proxying));

  function rename(part: "id" | "label", typed: string): void {
    setDraft(part, naming(typed));
    onRenamed();
  }
</script>

{#snippet pairTable(props: PairTableProps)}
  <!-- A row keeps its boxes - and its `key`, which is what the list is
  keyed by - when another row is removed, so the box a person is typing
  in never becomes somebody else's row. A row with no name is left out
  where the value is read, because the form keeps an empty row open
  while somebody types into it and a half-written header must not
  reach a provider. -->
  <div class="flex flex-col gap-tight text-note text-text-quiet">
    {props.title}
    {#each props.rows as row (row.key)}
      <div class="flex items-center gap-tight">
        <Field
          label={props.name}
          labelling="hidden"
          mono
          value={row.name}
          onInput={(text) => {
            props.change(props.rows.map((each) => (each.key === row.key ? { ...each, name: text } : each)));
          }}
        />
        <Field
          label={props.value}
          labelling="hidden"
          mono
          value={row.value}
          onInput={(text) => {
            props.change(props.rows.map((each) => (each.key === row.key ? { ...each, value: text } : each)));
          }}
        />
        <button
          type="button"
          class="h-control-sm shrink-0 rounded-control px-tight text-label text-text-quiet hover:bg-raised"
          aria-label={say($lang, "setup_remove_row")}
          onclick={() => {
            props.change(props.rows.filter((each) => each.key !== row.key));
          }}
        >
          ×
        </button>
      </div>
    {/each}
    <Button
      label={say($lang, "setup_add_row")}
      tone="quiet"
      onPress={() => {
        props.change([...props.rows, blankLine()]);
      }}
    />
  </div>
{/snippet}

<details class="flex flex-col gap-snug rounded-card bg-chrome px-base py-snug">
  <summary
    class="w-fit cursor-pointer rounded-control px-snug py-tight text-label text-text-quiet hover:bg-raised"
  >
    {say($lang, "setup_advanced")}
  </summary>
  <div class="grid grid-cols-[repeat(auto-fit,minmax(320px,1fr))] gap-snug">
    <Field
      label={say($lang, "setup_id")}
      help={say($lang, "setup_id_help")}
      mono
      pattern={ID_SHAPE.source}
      value={boxed(draft.id)}
      placeholder={idOf(draft)}
      onInput={(value: string) => {
        rename("id", value);
      }}
    />
    <Field
      label={say($lang, "setup_display_name")}
      help={say($lang, "setup_display_name_help")}
      value={boxed(draft.label)}
      placeholder={idOf(draft)}
      onInput={(value: string) => {
        rename("label", value);
      }}
    />
  </div>
  <div class="grid grid-cols-[repeat(auto-fit,minmax(320px,1fr))] gap-snug">
    {#each TUNING as [word, part, step] (part)}
      <Field
        label={say($lang, word)}
        kind="number"
        {step}
        mono
        value={draft[part]}
        onInput={(value: string) => {
          setDraft(part, value);
        }}
      />
    {/each}
  </div>
  <div class="flex flex-col gap-tight text-note text-text-quiet">
    {say($lang, "setup_proxying")}
    <Segmented
      label={say($lang, "setup_proxying")}
      options={PROXYINGS.map(([setting, word]) => ({ value: setting, label: say($lang, word) }))}
      held={draft.proxying}
      onPick={(rule: Proxying) => {
        setDraft("proxying", rule);
      }}
    />
    <span class="text-text-faint">{say($lang, "setup_proxying_help")}</span>
    {#if ruleNote !== undefined}
      <span class="text-text-faint">{say($lang, ruleNote)}</span>
    {/if}
  </div>
  <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
  {@render pairTable({
    title: say($lang, "setup_headers"),
    name: say($lang, "setup_header_name"),
    value: say($lang, "setup_header_value"),
    rows: draft.headers,
    change: (rows) => {
      setDraft("headers", [...rows]);
    },
  })}
  <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
  {@render pairTable({
    title: say($lang, "setup_overrides"),
    name: say($lang, "setup_pointer"),
    value: say($lang, "setup_override_value"),
    rows: draft.overrides,
    change: (rows) => {
      setDraft("overrides", [...rows]);
    },
  })}
</details>

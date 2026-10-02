<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The other half of choosing a model: what an endpoint answered when
  // it was asked for its model list, narrowed to the rows a person
  // ticked. An endpoint that serves two hundred rows, most of them
  // video and speech, is not a list anybody reads - it is a list
  // somebody filters.
  //
  // The table itself is `parts/table.svelte`: the sticky header, the
  // ordering, the tick column and the scroll box are that component's,
  // and what stays here is what only this page knows - which rows are
  // worth offering, what each column means, and which figure a row is
  // called with when nobody typed one.
  import { fill, say } from "../../core/lang";
  import type { ModelFact } from "../../core/probed";
  import { InputKinds as InputKindsSchema } from "../../wire";
  import type { InputKinds, ModelTag } from "../../wire";
  import { ui } from "../../ui";
  import Combobox from "../parts/combobox.svelte";
  import Field from "../parts/field.svelte";
  import { Table, type Column } from "../parts/table";
  import type { ModelRow, ModelTableProps } from "./models";

  const { lang } = ui();
  const { served, onChosen }: ModelTableProps = $props();

  // wording-ok: model ids, which are the same letters in every language.
  const MANUAL_PLACEHOLDER = "gpt-5-mini, claude-sonnet-4";

  // The input kinds a person may state, in the wire's own order.
  const INPUTS: readonly InputKinds[] = InputKindsSchema.literals;

  interface Filled {
    ticked: Record<string, boolean>;
    context: Record<string, string>;
    output: Record<string, string>;
    role: Record<string, string>;
    input: Record<string, InputKinds | null>;
  }

  // Mutated through its properties only, so the binding itself is
  // never reassigned.
  const filled = $state<Filled>({ ticked: {}, context: {}, output: {}, role: {}, input: {} });
  let search = $state<string>("");
  let textOnly = $state<boolean>(true);
  let manual = $state<string>("");

  // Word fragments that mark a model as something other than text in,
  // text out. A heuristic over an id, used only for the rows whose own
  // answer said nothing: a provider that states its input modalities is
  // believed, and a name that looks like a video model is not overruled
  // by a guess. It hides rows rather than refusing them, and the filter
  // is a switch a person can turn off.
  const NOT_TEXT: readonly string[] = [
    "asr",
    "audio",
    "diffusion",
    "embed",
    "image",
    "imagen",
    "flux",
    "rerank",
    "sora",
    "speech",
    "tts",
    "veo",
    "video",
    "vision",
    "whisper",
  ];

  function looksTextual(row: ModelFact): boolean {
    if (row.inputModalities.length > 0) return row.inputModalities.includes("text");
    const lower = row.id.toLowerCase();
    return !NOT_TEXT.some((mark) => lower.includes(mark));
  }

  // A whole positive number, or nothing. An empty box and a box holding
  // letters both mean "nobody stated this", which is what the wire
  // calls absent.
  function stated(text: string): number | null {
    const trimmed = text.trim();
    if (trimmed === "" || !/^[0-9]+$/.test(trimmed)) return null;
    const figure = Number.parseInt(trimmed, 10);
    return figure > 0 ? figure : null;
  }

  function tagOf(value: string): ModelTag | null {
    switch (value) {
      case "main":
      case "digest":
      case "transcribe":
        return value;
      default:
        // Anything else is the box a person left alone: no role.
        return null;
    }
  }

  // What an untouched box shows: the figure the provider itself stated,
  // drawn as a placeholder rather than as a value, because a placeholder
  // says "this is what will be sent" where a value would claim a person
  // decided it. A provider that stated nothing leaves the box blank.
  function placeholderOf(stated: number | null): string {
    return stated === null ? "" : String(stated);
  }

  // The two prices as the provider printed them, in and out. A row that
  // priced neither shows the same dash every unstated figure shows.
  // wording-ok: the dash is punctuation - it stands for a figure the
  // provider never printed, and reads the same in either language.
  function priceOf(row: ModelFact): string {
    if (row.inputPrice === null && row.outputPrice === null) return "—";
    return `${row.inputPrice ?? "—"} / ${row.outputPrice ?? "—"}`;
  }

  // A row that states no figure sorts below every row that does,
  // because the question this column is ordered to answer is which of
  // them is the biggest.
  function bySize(a: number | null, b: number | null): number {
    return (a ?? 0) - (b ?? 0);
  }

  // The ids the person typed for an endpoint that serves no list, or
  // serves one this city could not read.
  const named = $derived.by((): readonly ModelFact[] =>
    manual
      .split(",")
      .map((each) => each.trim())
      .filter((each) => each !== "")
      .map((id) => ({
        id,
        contextTokens: null,
        maxOutputTokens: null,
        inputModalities: [],
        inputPrice: null,
        outputPrice: null,
      })),
  );
  const every = $derived.by((): readonly ModelFact[] => {
    const all = [...served, ...named];
    return all.filter((row, at) => all.findIndex((other) => other.id === row.id) === at);
  });
  const byHand = $derived(named.map((row) => row.id));
  // Ordered by id, which stands one vendor's rows together because the
  // vendor is the part of an id before the slash. Any column that can
  // be ordered by reorders it from there; this is where it opens.
  const shown = $derived.by((): readonly ModelFact[] => {
    const needle = search.trim().toLowerCase();
    return every
      .filter(
        (row) =>
          (needle === "" || row.id.toLowerCase().includes(needle)) &&
          (!textOnly || looksTextual(row) || byHand.includes(row.id)),
      )
      .sort((a, b) => a.id.localeCompare(b.id));
  });
  const ticked = $derived(every.filter((row) => filled.ticked[row.id] === true));
  // The figures a row is called with: what the person typed, and
  // otherwise what the provider stated. An empty box therefore means
  // "keep what the provider stated" rather than "state nothing", which
  // is why that figure is drawn as a placeholder and not as a value.
  function windowOf(row: ModelFact): number | null {
    return stated(filled.context[row.id] ?? "") ?? row.contextTokens;
  }
  function ceilingOf(row: ModelFact): number | null {
    return stated(filled.output[row.id] ?? "") ?? row.maxOutputTokens;
  }
  const rows = $derived<readonly ModelRow[]>(
    ticked.map((row) => ({
      id: row.id,
      stated: { contextTokens: windowOf(row), maxOutputTokens: ceilingOf(row), input: filled.input[row.id] ?? null },
      tag: tagOf(filled.role[row.id] ?? ""),
    })),
  );

  function tickAll(on: boolean): void {
    for (const row of shown) filled.ticked[row.id] = on;
  }

  // The role column's choices: the three roles, and the one that takes
  // a role back off a row.
  const roleChoices = $derived([
    { value: "", label: say($lang, "setup_role_none") },
    { value: "main", label: say($lang, "setup_main") },
    { value: "digest", label: say($lang, "setup_digest") },
    { value: "transcribe", label: say($lang, "setup_transcribe") },
  ]);

  // Every column states what it may not be drawn narrower than, and
  // nothing in a cell is broken across lines to make it fit. The id
  // column is the one that must never break: a model id is the whole of
  // what a person ticks, and the columns together are wider than the
  // measure this table sits in, so the container scrolls sideways
  // rather than squeezing the id into a vertical word (client-SPEC
  // 4-33). Column order: id, context, output ceiling, input modalities,
  // price, role.
  const columns: readonly Column<ModelFact>[] = $derived([
    {
      key: "id",
      header: say($lang, "setup_model_id"),
      min: "24ch",
      render: renderId,
      compare: (a: ModelFact, b: ModelFact) => a.id.localeCompare(b.id),
    },
    {
      key: "context",
      header: say($lang, "setup_model_context"),
      min: "figure",
      render: renderWindow,
      compare: (a: ModelFact, b: ModelFact) => bySize(windowOf(a), windowOf(b)),
    },
    {
      key: "output",
      header: say($lang, "setup_model_output"),
      min: "figure",
      render: renderCeiling,
      compare: (a: ModelFact, b: ModelFact) => bySize(ceilingOf(a), ceilingOf(b)),
    },
    {
      key: "modalities",
      header: say($lang, "setup_model_modalities"),
      min: "12ch",
      render: renderModalities,
    },
    {
      key: "price",
      header: say($lang, "setup_model_price"),
      min: "12ch",
      render: renderPrice,
    },
    {
      key: "role",
      header: say($lang, "setup_model_role"),
      min: "figure",
      render: renderRole,
    },
  ]);

  $effect(() => {
    onChosen(rows);
  });
</script>

{#snippet renderId(row: ModelFact)}
  <span class="block whitespace-nowrap font-mono text-note text-text">{row.id}</span>
{/snippet}

{#snippet renderWindow(row: ModelFact)}
  <Field
    label={`${say($lang, "setup_model_context")} ${row.id}`}
    labelling="hidden"
    kind="number"
    step={1024}
    mono
    value={filled.context[row.id] ?? ""}
    placeholder={placeholderOf(row.contextTokens)}
    onInput={(value) => {
      filled.context[row.id] = value;
    }}
  />
{/snippet}

{#snippet renderCeiling(row: ModelFact)}
  <Field
    label={`${say($lang, "setup_model_output")} ${row.id}`}
    labelling="hidden"
    kind="number"
    step={1024}
    mono
    value={filled.output[row.id] ?? ""}
    placeholder={placeholderOf(row.maxOutputTokens)}
    onInput={(value) => {
      filled.output[row.id] = value;
    }}
  />
{/snippet}

<!-- What the model takes as input, as the person states it (X6): a
     model that reads pictures is the one a picture may be sent to, and
     an unstated row is the city's to look up. The provider's own list,
     when it gave one, stands under the choice as the evidence. -->
{#snippet renderModalities(row: ModelFact)}
  <div class="flex flex-col gap-hair">
    <select
      class="h-control-sm min-w-0 rounded-control border border-edge-input bg-raised px-snug text-note text-text"
      aria-label={`${say($lang, "setup_model_modalities")} ${row.id}`}
      value={filled.input[row.id] ?? ""}
      onchange={(event) => {
        const picked = INPUTS.find((each) => each === event.currentTarget.value);
        filled.input[row.id] = picked ?? null;
      }}
    >
      <option value="">{say($lang, "setup_input_unstated")}</option>
      {#each INPUTS as each (each)}
        <option value={each}>{say($lang, `setup_input_${each}`)}</option>
      {/each}
    </select>
    <!-- wording-ok: the modalities the provider itself stated, spelled
         as the provider spelled them -->
    <span class="text-note text-text-faint">{row.inputModalities.join(" ")}</span>
  </div>
{/snippet}

{#snippet renderPrice(row: ModelFact)}
  <span class="block whitespace-nowrap font-mono text-note text-text-faint">{priceOf(row)}</span>
{/snippet}

{#snippet renderRole(row: ModelFact)}
  <Combobox
    label={`${say($lang, "setup_model_role")} ${row.id}`}
    placeholder={say($lang, "part_search")}
    empty={say($lang, "part_no_match")}
    choices={roleChoices}
    value={filled.role[row.id] ?? null}
    onPick={(value: string) => {
      filled.role[row.id] = value;
    }}
  />
{/snippet}

{#snippet emptyRows()}
  <p class="px-base py-snug text-text-faint">{say($lang, "setup_no_models")}</p>
{/snippet}

<div class="flex flex-col gap-snug text-note">
  <!-- One row above the table, and it stays one row: the search box
       takes what the filter does not need, so a narrow column makes the
       box shorter rather than dropping the switch under it. -->
  <div class="flex items-center gap-snug">
    <div class="min-w-0 flex-1">
      <Field
        label={say($lang, "setup_model_search")}
        labelling="hidden"
        value={search}
        placeholder={say($lang, "setup_model_search")}
        onInput={(value) => {
          search = value;
        }}
      />
    </div>
    <label class="flex shrink-0 items-center gap-tight whitespace-nowrap text-text-quiet">
      <input type="checkbox" bind:checked={textOnly} />
      {say($lang, "setup_text_only")}
    </label>
  </div>
  <p class="text-text-faint">
    {fill(say($lang, "setup_ticked_count"), {
      ticked: String(ticked.length),
      total: String(every.length),
    })}
  </p>
  {#if every.length > 0}
    <Table
      caption={say($lang, "setup_models")}
      {columns}
      rows={shown}
      keyOf={(row: ModelFact) => row.id}
      empty={emptyRows}
      selection={{
        picked: (row: ModelFact) => filled.ticked[row.id] === true,
        onPick: (row: ModelFact, on: boolean) => {
          filled.ticked[row.id] = on;
        },
        allLabel: say($lang, "setup_select_all"),
        allPicked: () => shown.length > 0 && shown.every((row) => filled.ticked[row.id] === true),
        onPickAll: tickAll,
      }}
    />
  {/if}
  <!-- Not an alarm. The ceiling ladder in the city always answers
       (`gateway::OutputCeiling::resolve`), so an empty box is a decision
       left to the city rather than a call that will be refused, and the
       line says which rungs will answer it. -->
  {#if ticked.some((row) => ceilingOf(row) === null)}
    <p class="text-text-faint">{say($lang, "setup_model_needed")}</p>
  {/if}
  <Field
    label={say($lang, "setup_manual_ids")}
    mono
    value={manual}
    placeholder={MANUAL_PLACEHOLDER}
    onInput={(value) => {
      manual = value;
    }}
  />
</div>

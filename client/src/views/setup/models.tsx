// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Which model does the thinking, and how hard by default. `main` is the
// one a dispatch is refused without; `digest` reads long documents on
// its behalf and follows `main` unless it is pointed elsewhere;
// `transcribe` turns a recording into a line of text, and a city with
// none draws no microphone.
//
// The table below is the other half: what an endpoint answered when it
// was asked for its model list, narrowed to the rows a person ticked.
// An endpoint that serves two hundred rows, most of them video and
// speech, is not a list anybody reads - it is a list somebody filters.
//
// The table itself is `parts/table.tsx`: the sticky header, the
// ordering, the tick column and the scroll box are that component's,
// and what stays here is what only this page knows - which rows are
// worth offering, what each column means, and which figure a row is
// called with when nobody typed one.

import { For, Show, createEffect, createMemo, createSignal } from "solid-js";
import { createStore } from "solid-js/store";

import { selectModel } from "../../core/commands";
import type { Ceilings } from "../../core/commands";
import type { ModelFact } from "../../core/probed";
import type { EndpointsAnswer, ModelTag } from "../../wire";
import { useCommand, useSay } from "../../ui";
import { Field } from "../parts/field";
import { Table, type Column } from "../parts/table";

// One ticked row: the model, the two ceilings a person read off the
// provider's own documentation, and the role it is to fill if any.
export interface ModelRow {
  readonly id: string;
  readonly ceilings: Ceilings;
  readonly tag: ModelTag | null;
}

// Word fragments that mark a model as something other than text in,
// text out. A heuristic over an id, used only for the rows whose own
// answer said nothing: a provider that states its input modalities is
// believed, and a name that looks like a video model is not overruled by
// a guess. It hides rows rather than refusing them, and the filter is a
// switch a person can turn off.
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
// letters both mean "nobody stated this", which is what the wire calls
// absent.
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
      return null;
  }
}

interface Filled {
  ticked: Record<string, boolean>;
  context: Record<string, string>;
  output: Record<string, string>;
  role: Record<string, string>;
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
function priceOf(row: ModelFact): string {
  if (row.inputPrice === null && row.outputPrice === null) return "—";
  return `${row.inputPrice ?? "—"} / ${row.outputPrice ?? "—"}`;
}

export function ModelTable(props: {
  readonly served: readonly ModelFact[];
  readonly onChosen: (rows: readonly ModelRow[]) => void;
}) {
  const say = useSay();
  const [filled, setFilled] = createStore<Filled>({ ticked: {}, context: {}, output: {}, role: {} });
  const [search, setSearch] = createSignal("");
  const [textOnly, setTextOnly] = createSignal(true);
  const [manual, setManual] = createSignal("");

  // The ids the person typed for an endpoint that serves no list, or
  // serves one this city could not read.
  const named = createMemo<readonly ModelFact[]>(() =>
    manual()
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
  const every = createMemo<readonly ModelFact[]>(() => {
    const all = [...props.served, ...named()];
    return all.filter((row, at) => all.findIndex((other) => other.id === row.id) === at);
  });
  const byHand = createMemo(() => named().map((row) => row.id));
  // Ordered by id, which stands one vendor's rows together because the
  // vendor is the part of an id before the slash. Any column that can
  // be ordered by reorders it from there; this is where it opens.
  const shown = createMemo(() => {
    const needle = search().trim().toLowerCase();
    return every()
      .filter(
        (row) =>
          (needle === "" || row.id.toLowerCase().includes(needle)) &&
          (!textOnly() || looksTextual(row) || byHand().includes(row.id)),
      )
      .sort((a, b) => a.id.localeCompare(b.id));
  });
  const ticked = createMemo(() => every().filter((row) => filled.ticked[row.id] === true));
  // The figures a row is called with: what the person typed, and
  // otherwise what the provider stated. An empty box therefore means
  // "keep what the provider stated" rather than "state nothing", which
  // is why that figure is drawn as a placeholder and not as a value.
  const windowOf = (row: ModelFact): number | null =>
    stated(filled.context[row.id] ?? "") ?? row.contextTokens;
  const ceilingOf = (row: ModelFact): number | null =>
    stated(filled.output[row.id] ?? "") ?? row.maxOutputTokens;
  const rows = createMemo<readonly ModelRow[]>(() =>
    ticked().map((row) => ({
      id: row.id,
      ceilings: { contextTokens: windowOf(row), maxOutputTokens: ceilingOf(row) },
      tag: tagOf(filled.role[row.id] ?? ""),
    })),
  );
  createEffect(() => {
    props.onChosen(rows());
  });

  const tickAll = (state: boolean) => {
    for (const row of shown()) setFilled("ticked", row.id, state);
  };

  // A row that states no figure sorts below every row that does,
  // because the question this column is ordered to answer is which of
  // them is the biggest.
  const bySize = (a: number | null, b: number | null): number => (a ?? 0) - (b ?? 0);
  // Only the id column grows. It takes whatever width the others
  // leave and breaks a long id anywhere rather than cutting it off,
  // because an id a person cannot read whole is an id they cannot tick
  // with confidence.
  //
  // Every other column therefore states a bound, in one of the two
  // ways this table needs: a control sits in a `w-figure` box, and
  // text is left free to wrap. A cell that states neither is the cell
  // that takes the width away from the id column - which is what
  // `whitespace-nowrap` did in the modalities and price columns, and
  // what a `<select>` does by being as wide as its longest option
  // however narrow the table is.
  const columns = createMemo<readonly Column<ModelFact>[]>(() => [
    {
      key: "id",
      header: say("setup_model_id"),
      render: (row) => <span class="block min-w-0 font-mono text-note wrap-anywhere text-text">{row.id}</span>,
      compare: (a, b) => a.id.localeCompare(b.id),
    },
    {
      key: "context",
      header: say("setup_model_context"),
      render: (row) => (
        <div class="w-figure">
          <Field
            label={`${say("setup_model_context")} ${row.id}`}
            labelling="hidden"
            kind="number"
            step={1024}
            mono
            value={filled.context[row.id] ?? ""}
            placeholder={placeholderOf(row.contextTokens)}
            onInput={(value) => { setFilled("context", row.id, value); }}
          />
        </div>
      ),
      compare: (a, b) => bySize(windowOf(a), windowOf(b)),
    },
    {
      key: "output",
      header: say("setup_model_output"),
      render: (row) => (
        <div class="w-figure">
          <Field
            label={`${say("setup_model_output")} ${row.id}`}
            labelling="hidden"
            kind="number"
            step={1024}
            mono
            value={filled.output[row.id] ?? ""}
            placeholder={placeholderOf(row.maxOutputTokens)}
            onInput={(value) => { setFilled("output", row.id, value); }}
          />
        </div>
      ),
      compare: (a, b) => bySize(ceilingOf(a), ceilingOf(b)),
    },
    {
      key: "modalities",
      header: say("setup_model_modalities"),
      // wording-ok: the modalities and the prices the provider itself stated
      render: (row) => (
        <span class="text-note text-text-faint">
          {row.inputModalities.length === 0 ? "—" : row.inputModalities.join(" ")}
        </span>
      ),
    },
    {
      key: "price",
      header: say("setup_model_price"),
      render: (row) => <span class="font-mono text-note text-text-faint">{priceOf(row)}</span>,
    },
    {
      key: "role",
      header: say("setup_model_role"),
      render: (row) => (
        <div class="w-figure">
          <select
            class="w-full rounded-control bg-raised px-snug py-tight text-note text-text"
            value={filled.role[row.id] ?? ""}
            aria-label={`${say("setup_model_role")} ${row.id}`}
            onChange={(event) => { setFilled("role", row.id, event.currentTarget.value); }}
          >
            <option value="">—</option>
            <option value="main">{say("setup_main")}</option>
            <option value="digest">{say("setup_digest")}</option>
            <option value="transcribe">{say("setup_transcribe")}</option>
          </select>
        </div>
      ),
    },
  ]);

  return (
    <div class="flex flex-col gap-snug text-note">
      <div class="flex flex-wrap items-center gap-snug">
        <div class="min-w-0 flex-1">
          <Field
            label={say("setup_model_search")}
            labelling="hidden"
            value={search()}
            placeholder={say("setup_model_search")}
            onInput={setSearch}
          />
        </div>
        <label class="flex items-center gap-tight text-text-quiet">
          <input type="checkbox" checked={textOnly()} onChange={(event) => setTextOnly(event.currentTarget.checked)} />
          {say("setup_text_only")}
        </label>
      </div>
      <p class="text-text-faint">
        {say("setup_ticked_count", { ticked: String(ticked().length), total: String(every().length) })}
      </p>
      <Show when={every().length > 0}>
        <Table
          caption={say("setup_models")}
          columns={columns()}
          rows={shown()}
          keyOf={(row) => row.id}
          empty={<p class="px-base py-snug text-text-faint">{say("setup_no_models")}</p>}
          selection={{
            picked: (row) => filled.ticked[row.id] === true,
            onPick: (row, on) => { setFilled("ticked", row.id, on); },
            allLabel: say("setup_select_all"),
            allPicked: () => shown().length > 0 && shown().every((row) => filled.ticked[row.id] === true),
            onPickAll: (on) => { tickAll(on); },
          }}
        />
      </Show>
      {/* Not an alarm. The ceiling ladder in the city always answers
          (`gateway::OutputCeiling::resolve`), so an empty box is a
          decision left to the city rather than a call that will be
          refused, and the line says which rungs will answer it. */}
      <Show when={ticked().some((row) => ceilingOf(row) === null)}>
        <p class="text-text-faint">{say("setup_model_needed")}</p>
      </Show>
      <Field
        label={say("setup_manual_ids")}
        mono
        value={manual()}
        // wording-ok: model ids, which are the same letters in every language
        placeholder="gpt-5-mini, claude-sonnet-4"
        onInput={setManual}
      />
    </div>
  );
}

export function ModelChoice(props: { readonly answer: EndpointsAnswer; readonly tags?: readonly ModelTag[] }) {
  const say = useSay();
  const command = useCommand();
  const tags = () => props.tags ?? (["main", "digest", "transcribe"] as const);
  // An endpoint is carried by the id a command names it with and read
  // by the name the person gave it. The label is never empty: an
  // endpoint nobody named is labelled with its own id by the city
  // (`channels::EndpointSummary`), so this list needs no fallback.
  const options = createMemo(() =>
    props.answer.endpoints.flatMap((endpoint) =>
      endpoint.models.map((row) => ({
        endpoint: endpoint.name,
        label: endpoint.label,
        model: row.id,
      })),
    ),
  );
  const chosen = (tag: ModelTag) => props.answer.chosen.find((each) => each.tag === tag);
  const value = (tag: ModelTag) => {
    const held = chosen(tag);
    return held === undefined ? "" : `${held.endpoint}\u0000${held.model}`;
  };
  return (
    <div class="flex flex-col gap-base">
      <Show when={options().length > 0} fallback={<p class="text-note text-text-faint">{say("setup_no_models")}</p>}>
        <For each={tags()}>
          {(tag) => (
            <label class="flex flex-col gap-tight text-note text-text-quiet">
              {say(`setup_${tag}`)}
              <select
                class="rounded-control bg-raised px-base py-snug text-body text-text"
                value={value(tag)}
                onChange={(event) => {
                  const [endpoint, model] = event.currentTarget.value.split("\u0000");
                  if (endpoint !== undefined && model !== undefined && model !== "") {
                    command(selectModel(endpoint, model, tag));
                    if (tag === "main" && chosen("digest") === undefined) {
                      command(selectModel(endpoint, model, "digest"));
                    }
                  }
                }}
              >
                <option value="">—</option>
                <For each={options()}>
                  {(option) => (
                    <option value={`${option.endpoint}\u0000${option.model}`}>
                      {option.model} · {option.label}
                    </option>
                  )}
                </For>
              </select>
            </label>
          )}
        </For>
      </Show>
    </div>
  );
}

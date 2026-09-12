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

import { For, Show, createEffect, createMemo, createSignal } from "solid-js";
import { createStore } from "solid-js/store";

import { selectModel } from "../../core/commands";
import type { Ceilings } from "../../core/commands";
import type { ModelFact } from "../../core/probed";
import { EFFORTS } from "../../core/prefs";
import type { EndpointsAnswer, ModelTag } from "../../wire";
import { useCommand, useSay, useUi } from "../../ui";

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

// The part of a model id before the slash, which every catalogue that
// carries more than one vendor uses to say whose model this is.
function vendorOf(id: string): string | null {
  const cut = id.indexOf("/");
  return cut > 0 ? id.slice(0, cut) : null;
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

// A figure the provider itself stated, as the box shows it. An empty
// string is what "nobody stated this" looks like in an input, and the
// person filling it in is the next authority.
function prefilled(stated: number | null): string {
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
  const shown = createMemo(() => {
    const needle = search().trim().toLowerCase();
    return every().filter(
      (row) =>
        (needle === "" || row.id.toLowerCase().includes(needle)) &&
        (!textOnly() || looksTextual(row) || byHand().includes(row.id)),
    );
  });
  const vendors = createMemo(() => {
    const groups = new Map<string, ModelFact[]>();
    for (const row of shown()) {
      const vendor = vendorOf(row.id) ?? "";
      const held = groups.get(vendor);
      if (held === undefined) groups.set(vendor, [row]);
      else held.push(row);
    }
    return [...groups.entries()];
  });
  const ticked = createMemo(() => every().filter((row) => filled.ticked[row.id] === true));
  // What the provider stated, unless the person typed over it. The two
  // boxes start prefilled from the row's own answer, so a table nobody
  // touched still carries the figures the provider gave.
  const rows = createMemo<readonly ModelRow[]>(() =>
    ticked().map((row) => ({
      id: row.id,
      ceilings: {
        contextTokens: stated(filled.context[row.id] ?? "") ?? row.contextTokens,
        maxOutputTokens: stated(filled.output[row.id] ?? "") ?? row.maxOutputTokens,
      },
      tag: tagOf(filled.role[row.id] ?? ""),
    })),
  );
  createEffect(() => {
    props.onChosen(rows());
  });

  const tickAll = (state: boolean) => {
    for (const row of shown()) setFilled("ticked", row.id, state);
  };

  return (
    <div class="flex flex-col gap-snug text-note">
      <div class="flex flex-wrap items-center gap-snug">
        <input
          class="min-w-0 flex-1 rounded-control bg-g2 px-base py-snug text-body text-text outline-none"
          value={search()}
          aria-label={say("setup_model_search")}
          placeholder={say("setup_model_search")}
          onInput={(event) => setSearch(event.currentTarget.value)}
        />
        <label class="flex items-center gap-tight text-text-quiet">
          <input type="checkbox" checked={textOnly()} onChange={(event) => setTextOnly(event.currentTarget.checked)} />
          {say("setup_text_only")}
        </label>
        <button
          type="button"
          class="rounded-control bg-g2 px-base py-tight text-label hover:bg-g3"
          onClick={() => { tickAll(true); }}
        >
          {say("setup_select_all")}
        </button>
        <button
          type="button"
          class="rounded-control bg-g2 px-base py-tight text-label hover:bg-g3"
          onClick={() => { tickAll(false); }}
        >
          {say("setup_select_none")}
        </button>
      </div>
      <p class="text-text-faint">
        {say("setup_ticked_count", { ticked: String(ticked().length), total: String(every().length) })}
      </p>
      <Show when={every().length > 0}>
        <div class="max-h-output overflow-auto rounded-card bg-g1 px-snug py-snug">
          <table class="w-full table-fixed border-collapse text-left">
            <thead class="text-text-faint">
              <tr>
                <th class="w-glyph" />
                <th class="py-tight font-label">{say("setup_model_id")}</th>
                <th class="w-figure py-tight font-label">{say("setup_model_context")}</th>
                <th class="w-figure py-tight font-label">{say("setup_model_output")}</th>
                <th class="w-figure py-tight font-label">{say("setup_model_modalities")}</th>
                <th class="w-figure py-tight font-label">{say("setup_model_price")}</th>
                <th class="w-figure py-tight font-label">{say("setup_model_role")}</th>
              </tr>
            </thead>
            <For each={vendors()}>
              {([vendor, models]) => (
                <tbody>
                  <tr>
                    <td colspan="7" class="pt-snug text-text-faint">
                      {vendor === "" ? say("setup_vendor_none") : vendor}
                    </td>
                  </tr>
                  <For each={models}>
                    {(row) => (
                      <tr class="align-middle">
                        <td class="py-tight">
                          <input
                            type="checkbox"
                            checked={filled.ticked[row.id] === true}
                            aria-label={say("setup_tick_model", { model: row.id })}
                            onChange={(event) => { setFilled("ticked", row.id, event.currentTarget.checked); }}
                          />
                        </td>
                        <td class="truncate py-tight font-mono text-text" title={row.id}>
                          {row.id}
                        </td>
                        <td class="py-tight">
                          <input
                            class="w-full rounded-control bg-g2 px-snug py-tight font-mono text-note text-text outline-none"
                            value={filled.context[row.id] ?? prefilled(row.contextTokens)}
                            inputmode="numeric"
                            aria-label={say("setup_model_context")}
                            onInput={(event) => { setFilled("context", row.id, event.currentTarget.value); }}
                          />
                        </td>
                        <td class="py-tight">
                          <input
                            class="w-full rounded-control bg-g2 px-snug py-tight font-mono text-note text-text outline-none"
                            value={filled.output[row.id] ?? prefilled(row.maxOutputTokens)}
                            inputmode="numeric"
                            aria-label={say("setup_model_output")}
                            onInput={(event) => { setFilled("output", row.id, event.currentTarget.value); }}
                          />
                        </td>
                        {/* wording-ok: the modalities and the prices the provider itself stated */}
                        <td class="truncate py-tight text-text-faint" title={row.inputModalities.join(" ")}>
                          {row.inputModalities.length === 0 ? "—" : row.inputModalities.join(" ")}
                        </td>
                        <td class="truncate py-tight font-mono text-text-faint">
                          {priceOf(row)}
                        </td>
                        <td class="py-tight">
                          <select
                            class="w-full rounded-control bg-g2 px-snug py-tight text-note text-text outline-none"
                            value={filled.role[row.id] ?? ""}
                            aria-label={say("setup_model_role")}
                            onChange={(event) => { setFilled("role", row.id, event.currentTarget.value); }}
                          >
                            <option value="">—</option>
                            <option value="main">{say("setup_main")}</option>
                            <option value="digest">{say("setup_digest")}</option>
                            <option value="transcribe">{say("setup_transcribe")}</option>
                          </select>
                        </td>
                      </tr>
                    )}
                  </For>
                </tbody>
              )}
            </For>
          </table>
        </div>
      </Show>
      <Show when={ticked().some((row) => (stated(filled.output[row.id] ?? "") ?? row.maxOutputTokens) === null)}>
        <p class="text-alert">{say("setup_model_needed")}</p>
      </Show>
      <label class="flex flex-col gap-tight text-text-quiet">
        {say("setup_manual_ids")}
        <input
          class="rounded-control bg-g2 px-base py-snug font-mono text-body text-text outline-none"
          value={manual()}
          // wording-ok: model ids, which are the same letters in every language
          placeholder="gpt-5-mini, claude-sonnet-4"
          onInput={(event) => setManual(event.currentTarget.value)}
        />
      </label>
    </div>
  );
}

export function ModelChoice(props: { readonly answer: EndpointsAnswer; readonly tags?: readonly ModelTag[] }) {
  const say = useSay();
  const command = useCommand();
  const tags = () => props.tags ?? (["main", "digest", "transcribe"] as const);
  const options = createMemo(() =>
    props.answer.endpoints.flatMap((endpoint) => endpoint.models.map((model) => ({ endpoint: endpoint.name, model }))),
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
                class="rounded-control bg-g2 px-base py-snug text-body text-text outline-none"
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
                      {option.model} · {option.endpoint}
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

export function EffortChoice() {
  const ui = useUi();
  const say = useSay();
  return (
    <div class="flex flex-col gap-tight text-note text-text-quiet">
      {say("setup_effort")}
      <div class="flex flex-wrap gap-tight">
        <For each={EFFORTS}>
          {(effort) => (
            <button
              type="button"
              class={`rounded-pill px-base py-tight text-label ${ui.prefs.effort() === effort ? "bg-accent text-g0" : "bg-g2 text-text-quiet hover:bg-g3"}`}
              onClick={() => {
                ui.prefs.setEffort(effort);
              }}
            >
              {say(`effort_${effort}`)}
            </button>
          )}
        </For>
      </div>
    </div>
  );
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the attach form holds, what each field of it means, and the
// boxes that stay folded away until somebody asks for them.
//
// **A provider is named by its URL.** The id is the key of
// `[model_providers.<id>]` and what `secret:providers/<id>` derives
// from, so a form that asks for it first asks somebody to invent this
// city's bookkeeping before they may paste the one thing they arrived
// with. The host already carries the name the provider is known by, so
// the id is derived from it and both naming boxes sit under
// `advanced`, where somebody who keeps a `config.toml` of their own
// overrules them.
//
// **The derivation is one rule and no host table.** The label before
// the last one is `openai` for `api.openai.com`, `anthropic` for
// `api.anthropic.com` and `openrouter` for `openrouter.ai`, so a table
// of known hosts on this side would be a second home for what
// `gateway::provider::preset` already holds, and the two would
// disagree the first time a provider moved.

import { For, Show } from "solid-js";
import type { SetStoreFunction } from "solid-js/store";

import { WIRE_APIS, dialectOf } from "../../../core/commands";
import type { Endpoint, Pair, Tuning, WireApi } from "../../../core/commands";
import type { Key } from "../../../core/lang";
import { browserRows, defaultProxying } from "../../../core/prefs";
import type { Proxying } from "../../../wire";
import { useSay } from "../../../ui";
import { Field } from "../../parts/field";
import { Segmented } from "../../parts/segmented";
import type { Choice, Group } from "../../parts/segmented";

// What a provider id may be spelled with, which is what a TOML table key
// and a secret reference can both carry unquoted.
export const ID_SHAPE = /^[a-z0-9][a-z0-9-]*$/;

// What a base URL must look like, spelled once for its two readers:
// the box's own `pattern`, which the browser checks while the person
// is still filling the form in, and `hostOf` below, which is what the
// look and attach controls are gated on.
//
// **`type="url"` is not that rule.** It accepts `mailto:somebody` and
// every other scheme, so a value this form refuses used to sit in a
// box the browser had called valid, and the person learnt of it by
// pressing a control that stayed grey without saying why.
//
// A provider states the root of its API and never the full path, so
// the shape is a scheme, an ASCII host, an optional port, an optional
// path, and nothing after it. A query string is refused because a
// root never carries one, and every face hangs its own path off this
// value.
export const BASE_URL = /^https?:\/\/([a-zA-Z0-9.-]+)(?::[0-9]+)?(?:\/[^\s?#]*)?$/;

// The host a base URL names, which is what a person reads a reachability
// report about. A URL this form would refuse has no host, and the form
// says so before it lets anybody look.
export function hostOf(baseUrl: string): string | null {
  return BASE_URL.exec(baseUrl.trim())?.[1] ?? null;
}

// A name this form derives and a person may overrule.
//
// A cleared box is `derived` rather than a typed empty string: somebody
// who empties the box is asking for the derivation back, which is what
// the placeholder in front of them is already showing.
export type Naming =
  | { readonly kind: "derived" }
  | { readonly kind: "typed"; readonly value: string };

// What a box holding this text means. Trimming happens where the value
// is read rather than here: a name is edited a character at a time, and
// a store that ate the space just typed would eat the word after it.
export function naming(typed: string): Naming {
  return typed.trim() === "" ? { kind: "derived" } : { kind: "typed", value: typed };
}

// The text a naming box shows. Empty for a derived name, so the
// placeholder beside it is what the person reads.
export function boxed(held: Naming): string {
  return held.kind === "typed" ? held.value : "";
}

// Everything the form holds. A store rather than a dozen signals: these
// values are read together by three previews and two commands, and a
// preview that reads eleven signals one at a time is eleven chances to
// forget one.
export interface Draft {
  id: Naming;
  label: Naming;
  baseUrl: string;
  wireApi: WireApi;
  key: string;
  timeoutMs: string;
  requestRetries: string;
  streamIdleMs: string;
  proxying: Proxying;
  headers: Pair[];
  overrides: Pair[];
}

// The name a host suggests, in the shape a TOML table key and a secret
// reference can both carry.
//
// The label before the last one is what a provider is known by. A host
// with no such label - a bare name, or an address - is its own name,
// spelled the way a table key can be, because `127.0.0.1` has a `0`
// where every other host has its name.
export function idFrom(host: string | null): string {
  if (host === null) return "";
  const lower = host.toLowerCase();
  const labels = lower.split(".");
  const second = labels.at(-2);
  const named = labels.every((label) => /^[0-9]+$/.test(label)) ? lower : second ?? lower;
  return named.replace(/[^a-z0-9-]+/g, "-").replace(/^-+/, "").replace(/-+$/, "");
}

// The id this draft is filed under: the one somebody typed, or the one
// its URL suggests.
export function idOf(draft: Draft): string {
  return draft.id.kind === "typed" ? draft.id.value.trim() : idFrom(hostOf(draft.baseUrl));
}

// The three settings, each with the word it is offered under and the
// sentence that says which machine it is right for. A table rather than
// three branches: the control draws itself from it, and a fourth
// setting would be a row.
export const PROXYINGS: readonly (readonly [Proxying, Key, Key])[] = [
  ["except_local", "setup_proxying_except_local", "setup_proxying_note_except_local"],
  ["always", "setup_proxying_always", "setup_proxying_note_always"],
  ["never", "setup_proxying_never", "setup_proxying_note_never"],
];

// The sentence that says which machine one rule is right for, which
// the network screen and this form both draw under the control.
export function proxyingNote(rule: Proxying): Key | undefined {
  return PROXYINGS.find(([setting]) => setting === rule)?.[2];
}

// The two laboratories whose request shapes this form offers, and the
// token each paints its cells with. The mapping lives here and nowhere
// else: no other screen colours anything by which laboratory it came
// from.
const OPEN_AI: Group = { label: "OpenAI", tone: "accent" };
const ANTHROPIC: Group = { label: "Anthropic", tone: "alert" };

// The three `wire_api` values as cells of one control, grouped by the
// laboratory that defined each shape.
//
// **`dialectOf` decides which cell can be chosen.** A value this city's
// wire carries no dialect for is refused under the pointer with the
// caller's sentence, so the reason arrives before the click instead of
// as a refusal after it, and the cell carries no second sentence in red
// anywhere else on the form.
//
// A cell is worded with the wire value itself, not out of the phrase
// table: each of the three is one token that reads the same in both
// languages, and a translated `wire_api` would be a value nobody can
// paste into a `config.toml`.
export function wireChoices(unsupported: string): readonly Choice<WireApi>[] {
  return WIRE_APIS.map((api) => ({
    value: api,
    label: api,
    group: api === "messages" ? ANTHROPIC : OPEN_AI,
    ...(dialectOf(api) === null ? { why: unsupported } : {}),
  }));
}

const FRESH: Omit<Draft, "proxying"> = {
  id: { kind: "derived" },
  label: { kind: "derived" },
  baseUrl: "",
  wireApi: "chat",
  key: "",
  timeoutMs: "60000",
  requestRetries: "4",
  streamIdleMs: "300000",
  headers: [],
  overrides: [],
};

// An empty form, carrying the proxy rule this machine was last told to
// start new endpoints with.
export function freshDraft(): Draft {
  return { ...FRESH, proxying: defaultProxying(browserRows()) };
}

// A box of digits, or nothing. An empty box and a box holding letters
// both mean "the city's own", which is what absence is on the wire.
function figureIn(text: string): number | null {
  const trimmed = text.trim();
  return /^[0-9]+$/.test(trimmed) ? Number.parseInt(trimmed, 10) : null;
}

// What the advanced section settled, as the two commands carry it.
export function tuningOf(draft: Draft): Tuning {
  return {
    // Absent unless somebody typed another name. The city falls back to
    // the id when no label was stated (`AttachedEndpoint::label`), so a
    // client that sent the id as the label would be a second home for
    // that fallback and would outlive a rename.
    label: draft.label.kind === "typed" ? draft.label.value : null,
    timeoutMs: figureIn(draft.timeoutMs),
    requestMaxRetries: figureIn(draft.requestRetries),
    streamIdleTimeoutMs: figureIn(draft.streamIdleMs),
    headers: draft.headers,
    overrides: draft.overrides,
    proxying: draft.proxying,
  };
}

// The endpoint this draft describes, carrying the reference the vault
// answered with. Nothing when the wire it names is one this city cannot
// call, which is the same fact the control was already refused under.
export function endpointOf(draft: Draft, secret: string | null): Endpoint | null {
  const speaks = dialectOf(draft.wireApi);
  return speaks === null
    ? null
    : {
        id: idOf(draft),
        baseUrl: draft.baseUrl.trim(),
        dialect: speaks,
        secret,
        authHeader: null,
        tuning: tuningOf(draft),
      };
}

function PairTable(props: {
  readonly title: string;
  readonly nameLabel: string;
  readonly valueLabel: string;
  readonly rows: readonly Pair[];
  readonly onChange: (rows: readonly Pair[]) => void;
}) {
  const say = useSay();
  const edit = (at: number, part: "name" | "value", value: string) => {
    props.onChange(props.rows.map((row, index) => (index === at ? { ...row, [part]: value } : row)));
  };
  return (
    <div class="flex flex-col gap-tight text-note text-text-quiet">
      {props.title}
      <For each={props.rows}>
        {(row, at) => (
          <div class="flex items-center gap-tight">
            <Field
              label={props.nameLabel}
              labelling="hidden"
              mono
              value={row.name}
              onInput={(value) => { edit(at(), "name", value); }}
            />
            <Field
              label={props.valueLabel}
              labelling="hidden"
              mono
              value={row.value}
              onInput={(value) => { edit(at(), "value", value); }}
            />
            <button
              type="button"
              class="rounded-control px-snug py-tight text-label text-text-quiet hover:bg-g2"
              aria-label={say("setup_remove_row")}
              onClick={() => {
                props.onChange(props.rows.filter((_row, index) => index !== at()));
              }}
            >
              ×
            </button>
          </div>
        )}
      </For>
      <button
        type="button"
        class="self-start rounded-control bg-g2 px-base py-tight text-label hover:bg-g3"
        onClick={() => {
          props.onChange([...props.rows, { name: "", value: "" }]);
        }}
      >
        {say("setup_add_row")}
      </button>
    </div>
  );
}

// Everything a person may state and almost nobody has to: what this
// provider is called, and the five settings a working endpoint never
// needs changed.
//
// The two naming boxes lead, because they are the two the form stopped
// asking for on the way in; each shows its derived value as the
// placeholder, so an empty box is a statement of what will be used
// rather than a question nobody answered.
export function AdvancedFields(props: {
  readonly draft: Draft;
  readonly setDraft: SetStoreFunction<Draft>;
  // A rename retires the last report: what it said was about the name
  // the form carried then.
  readonly onRenamed: () => void;
}) {
  const say = useSay();
  const rename = (part: "id" | "label", typed: string) => {
    props.setDraft(part, naming(typed));
    props.onRenamed();
  };
  return (
    <div class="flex flex-col gap-snug rounded-card bg-g1 px-base py-snug">
      <div class="grid gap-snug md:grid-cols-2">
        <Field
          label={say("setup_id")}
          help={say("setup_id_help")}
          mono
          pattern={ID_SHAPE.source}
          value={boxed(props.draft.id)}
          placeholder={idOf(props.draft)}
          onInput={(value) => { rename("id", value); }}
        />
        <Field
          label={say("setup_display_name")}
          help={say("setup_display_name_help")}
          value={boxed(props.draft.label)}
          placeholder={idOf(props.draft)}
          onInput={(value) => { rename("label", value); }}
        />
      </div>
      <div class="grid gap-snug md:grid-cols-3">
        <Field
          label={say("setup_timeout_ms")}
          kind="number"
          step={1000}
          mono
          value={props.draft.timeoutMs}
          onInput={(value) => { props.setDraft("timeoutMs", value); }}
        />
        <Field
          label={say("setup_request_retries")}
          kind="number"
          step={1}
          mono
          value={props.draft.requestRetries}
          onInput={(value) => { props.setDraft("requestRetries", value); }}
        />
        <Field
          label={say("setup_stream_idle")}
          kind="number"
          step={1000}
          mono
          value={props.draft.streamIdleMs}
          onInput={(value) => { props.setDraft("streamIdleMs", value); }}
        />
      </div>
      <div class="flex flex-col gap-tight text-note text-text-quiet">
        {say("setup_proxying")}
        <Segmented
          label={say("setup_proxying")}
          options={PROXYINGS.map(([setting, word]) => ({ value: setting, label: say(word) }))}
          held={props.draft.proxying}
          onPick={(proxying) => { props.setDraft("proxying", proxying); }}
        />
        <span class="text-text-faint">{say("setup_proxying_help")}</span>
        <Show when={proxyingNote(props.draft.proxying)}>
          {(note) => <span class="text-text-faint">{say(note())}</span>}
        </Show>
      </div>
      <PairTable
        title={say("setup_headers")}
        nameLabel={say("setup_header_name")}
        valueLabel={say("setup_header_value")}
        rows={props.draft.headers}
        onChange={(next) => { props.setDraft("headers", [...next]); }}
      />
      <PairTable
        title={say("setup_overrides")}
        nameLabel={say("setup_pointer")}
        valueLabel={say("setup_override_value")}
        rows={props.draft.overrides}
        onChange={(next) => { props.setDraft("overrides", [...next]); }}
      />
    </div>
  );
}

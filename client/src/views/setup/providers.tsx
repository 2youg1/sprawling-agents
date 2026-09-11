// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Turning a URL and a key into models a run can be given. The fields are
// Codex's `[model_providers.<id>]` under its own names, so somebody who
// has written that file reads this form without learning a second
// vocabulary, and the preview at the bottom shows the file this form
// would have written.
//
// The key goes to the vault by its own route and comes back as a
// reference. **The reference is held beside the id it was filed under**:
// the reference is derived from the id, so a reference kept alone
// outlives the name that makes it true, and the second provider a person
// enrolled used to inherit the first one's key.
//
// A subscription login is the other door: begin, approve in a browser,
// bring back the code.

import { For, Show, createEffect, createMemo, createSignal } from "solid-js";
import { createStore } from "solid-js/store";

import {
  WIRE_APIS,
  attachEndpoint,
  dialectOf,
  loginBegin,
  loginCode,
  probeEndpoint,
  selectModel,
} from "../../core/commands";
import type { Endpoint, WireApi } from "../../core/commands";
import { enrol, keyField, referenceFor, secretFor } from "../../core/enrol";
import type { Enrolment, StoredKey } from "../../core/enrol";
import type { Key } from "../../core/lang";
import type { AxCode, AxError, DialectKind, EndpointsAnswer } from "../../wire";
import { useCommand, useSay, useUi } from "../../ui";
import { ModelTable } from "./models";
import type { ModelRow } from "./models";

// What a provider id may be spelled with, which is what a TOML table key
// and a secret reference can both carry unquoted.
const ID_SHAPE = /^[a-z0-9][a-z0-9-]*$/;

// One row of either key-value table: a header the request carries, or a
// JSON pointer into the body it sends.
interface Pair {
  name: string;
  value: string;
}

// Everything the form holds. A store rather than a dozen signals: these
// values are read together by three previews and two commands, and a
// preview that reads eleven signals one at a time is eleven chances to
// forget one.
interface Draft {
  id: string;
  label: string;
  baseUrl: string;
  wireApi: WireApi;
  key: string;
  timeoutMs: string;
  requestRetries: string;
  streamRetries: string;
  streamIdleMs: string;
  headers: Pair[];
  overrides: Pair[];
}

const FRESH: Draft = {
  id: "",
  label: "",
  baseUrl: "",
  wireApi: "chat",
  key: "",
  timeoutMs: "60000",
  requestRetries: "4",
  streamRetries: "4",
  streamIdleMs: "300000",
  headers: [],
  overrides: [],
};

// The path a dialect's call goes to, which is also what the preview
// shows: the base URL is the root a provider states, never the full
// path, and this is the one place that knows what each wire adds to it.
function callPath(api: WireApi): string {
  switch (api) {
    case "chat":
      return "/chat/completions";
    case "responses":
      return "/responses";
    case "messages":
      return "/messages";
  }
}

function isRecord(held: unknown): held is Record<string, unknown> {
  return typeof held === "object" && held !== null && !Array.isArray(held);
}

// What a person typed into an override's value box, read as the JSON
// value it spells. Numbers, the two booleans and null are read as
// themselves; everything else is the string it looks like, which is what
// `/reasoning/effort` wants.
function jsonValue(text: string): unknown {
  const trimmed = text.trim();
  if (trimmed === "true") return true;
  if (trimmed === "false") return false;
  if (trimmed === "null") return null;
  if (/^-?[0-9]+(\.[0-9]+)?$/.test(trimmed)) return Number.parseFloat(trimmed);
  return trimmed;
}

// One JSON pointer written into a body, returning the body it produced.
// A segment whose parent is not an object replaces that parent: the
// preview's job is to show where an override lands, and a pointer into
// a number lands by making the number an object.
function withPointer(
  node: Record<string, unknown>,
  path: readonly string[],
  value: unknown,
): Record<string, unknown> {
  const [head, ...rest] = path;
  if (head === undefined) return node;
  if (rest.length === 0) return { ...node, [head]: value };
  const held = node[head];
  return { ...node, [head]: withPointer(isRecord(held) ? held : {}, rest, value) };
}

function pointerPath(pointer: string): readonly string[] {
  return pointer
    .split("/")
    .map((segment) => segment.trim())
    .filter((segment) => segment !== "");
}

function pairs(rows: readonly Pair[]): readonly Pair[] {
  return rows.filter((row) => row.name.trim() !== "");
}

// The `config.toml` table this form would have written, so somebody who
// keeps one can compare the two by eye.
function configToml(draft: Draft, reference: string): string {
  const lines = [
    `[model_providers.${draft.id.trim() === "" ? "<id>" : draft.id.trim()}]`,
    `name = ${JSON.stringify(draft.label.trim() === "" ? draft.id.trim() : draft.label.trim())}`,
    `base_url = ${JSON.stringify(draft.baseUrl.trim())}`,
    `wire_api = ${JSON.stringify(draft.wireApi)}`,
    `env_key = ${JSON.stringify(reference)}`,
    `request_max_retries = ${draft.requestRetries.trim()}`,
    `stream_max_retries = ${draft.streamRetries.trim()}`,
    `stream_idle_timeout_ms = ${draft.streamIdleMs.trim()}`,
    `timeout_ms = ${draft.timeoutMs.trim()}`,
  ];
  const headers = pairs(draft.headers);
  if (headers.length > 0) {
    const written = headers.map((row) => `${JSON.stringify(row.name.trim())} = ${JSON.stringify(row.value)}`);
    lines.push(`http_headers = { ${written.join(", ")} }`);
  }
  return lines.join("\n");
}

// The call this endpoint would make, headers and body, with every
// override already applied. One minimal conversation, because the
// question it answers is where a pointer lands rather than what a model
// would say.
function requestPreview(draft: Draft, reference: string, model: string): string {
  const url = `${draft.baseUrl.trim()}${callPath(draft.wireApi)}`;
  const header =
    draft.wireApi === "messages" ? `x-api-key: ${reference}` : `authorization: Bearer ${reference}`;
  const written = [
    `POST ${url === callPath(draft.wireApi) ? "<base_url>" : url}`,
    header,
    "content-type: application/json",
    ...pairs(draft.headers).map((row) => `${row.name.trim().toLowerCase()}: ${row.value}`),
  ];
  let body: Record<string, unknown> = {
    model,
    messages: [{ role: "user", content: "hello" }],
    stream: true,
  };
  for (const row of pairs(draft.overrides)) {
    body = withPointer(body, pointerPath(row.name), jsonValue(row.value));
  }
  return `${written.join("\n")}\n\n${JSON.stringify(body, null, 2)}`;
}

// What a person can do about a refusal, by the code it carries. The
// city's own `recovery` speaks to the runtime that must retry or fail
// over, so it is copied rather than shown; this table is the half
// written for the person filling in the form.
const NEXT_STEP: readonly (readonly [AxCode, Key])[] = [
  ["E_PROVIDER", "setup_next_provider"],
  ["E_TIMEOUT", "setup_next_timeout"],
  ["E_CREDENTIAL_MISSING", "setup_next_credential"],
  ["E_CONFIG_INVALID", "setup_next_config"],
  ["E_INVALID_ARGS", "setup_next_config"],
  ["E_ENDPOINT_DIALECT_UNSUPPORTED", "setup_next_dialect"],
  ["E_WIRE_MISMATCH", "setup_next_dialect"],
];

function nextStep(code: AxCode): Key {
  return NEXT_STEP.find(([known]) => known === code)?.[1] ?? "setup_next_other";
}

// The host a base URL names, which is what a person reads a reachability
// report about. An unparseable URL has no host, and the form says so
// before it lets anybody look.
function hostOf(baseUrl: string): string | null {
  const match = /^https?:\/\/([^/:?#]+)/.exec(baseUrl.trim());
  return match?.[1] ?? null;
}

function Reachability(props: { readonly error: AxError; readonly host: string }) {
  const say = useSay();
  return (
    <div role="alert" class="rounded-card border border-alert/50 bg-g1 px-base py-snug text-note">
      <p class="font-label text-alert">{say("setup_probe_failed", { host: props.host })}</p>
      <p class="mt-tight text-text-quiet">{say(nextStep(props.error.code))}</p>
      <div class="mt-snug flex items-center gap-snug text-text-faint">
        <span class="font-mono">{props.error.code}</span>
        <button
          type="button"
          class="rounded-control px-snug py-tight text-label text-text-quiet hover:bg-g2"
          onClick={() => {
            void navigator.clipboard.writeText(
              `${props.error.code}\n${props.error.action}\n${props.error.subject}\n${props.error.recovery}`,
            );
          }}
        >
          {say("setup_copy_details")}
        </button>
      </div>
    </div>
  );
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
            <input
              class="min-w-0 flex-1 rounded-control bg-g2 px-snug py-tight font-mono text-note text-text outline-none"
              value={row.name}
              aria-label={props.nameLabel}
              onInput={(event) => { edit(at(), "name", event.currentTarget.value); }}
            />
            <input
              class="min-w-0 flex-1 rounded-control bg-g2 px-snug py-tight font-mono text-note text-text outline-none"
              value={row.value}
              aria-label={props.valueLabel}
              onInput={(event) => { edit(at(), "value", event.currentTarget.value); }}
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

function Figure(props: {
  readonly label: string;
  readonly value: string;
  readonly onInput: (value: string) => void;
}) {
  return (
    <label class="flex flex-1 flex-col gap-tight text-note text-text-quiet">
      {props.label}
      <input
        class="rounded-control bg-g2 px-base py-snug font-mono text-body text-text outline-none"
        value={props.value}
        inputmode="numeric"
        onInput={(event) => { props.onInput(event.currentTarget.value); }}
      />
    </label>
  );
}

export function AttachForm(props: { readonly onAttached?: () => void }) {
  const ui = useUi();
  const say = useSay();
  const command = useCommand();
  const [draft, setDraft] = createStore<Draft>({ ...FRESH });
  // The reference the vault answered with, and the id it was filed
  // under. Never one without the other.
  const [held, setHeld] = createSignal<StoredKey | null>(null);
  const [note, setNote] = createSignal<string | null>(null);
  const [report, setReport] = createSignal<AxError | null>(null);
  const [looking, setLooking] = createSignal(false);
  const [busy, setBusy] = createSignal(false);
  const [rows, setRows] = createSignal<readonly ModelRow[]>([]);
  const [advanced, setAdvanced] = createSignal(false);

  const id = () => draft.id.trim();
  const field = () => keyField(held(), id());
  const reference = () => secretFor(held(), id()) ?? referenceOf(id());
  const answered = () => {
    const probed = ui.conn.belief.probed;
    return probed !== null && probed.name === id() ? probed.models : null;
  };
  const models = createMemo(() => answered() ?? []);
  const dialect = (): DialectKind | null => dialectOf(draft.wireApi);
  const complete = () => ID_SHAPE.test(id()) && hostOf(draft.baseUrl) !== null && dialect() !== null;

  // A refusal that arrives while this form is waiting for one is this
  // form's to show, beside the field it is about; the corner is for
  // refusals no open page is responsible for.
  createEffect(() => {
    const refused = ui.conn.belief.refusal;
    if (refused === null || !looking()) return;
    setReport(refused);
    setLooking(false);
    ui.conn.dismissRefusal();
  });
  // An endpoint that answered an empty list answered: the wait ends on
  // the answer rather than on its length.
  createEffect(() => {
    if (answered() !== null) setLooking(false);
  });

  // Every edit retires the last report: what it said was about the
  // fields as they were.
  const edit = (part: "id" | "label" | "baseUrl" | "key", value: string) => {
    setDraft(part, value);
    setReport(null);
    setNote(null);
  };

  const endpoint = (secret: string | null): Endpoint | null => {
    const speaks = dialect();
    return speaks === null
      ? null
      : { id: id(), baseUrl: draft.baseUrl.trim(), dialect: speaks, secret, authHeader: null };
  };

  // The key is enrolled on the action that needs it, under the id the
  // form says now. A blank box keeps whatever is already filed under
  // that id; a box with something in it replaces it.
  const withKey = (then: (e: Endpoint) => void) => {
    const typed = draft.key.trim();
    const keep = secretFor(held(), id());
    if (typed === "") {
      const built = endpoint(keep);
      if (built !== null) then(built);
      return;
    }
    setBusy(true);
    const { realm, name } = referenceFor(id());
    const settle = (outcome: Enrolment) => {
      setBusy(false);
      if (outcome.kind !== "stored") {
        setNote(outcome.reason);
        return;
      }
      setHeld({ provider: id(), reference: outcome.reference });
      setDraft("key", "");
      const built = endpoint(outcome.reference);
      if (built !== null) then(built);
    };
    void enrol(ui.origin, realm, name, typed).then(settle);
  };

  return (
    <form
      class="flex flex-col gap-base"
      onSubmit={(event) => {
        event.preventDefault();
      }}
    >
      <label class="flex flex-col gap-tight text-note text-text-quiet">
        {say("setup_id")}
        <input
          class="rounded-control bg-g2 px-base py-snug font-mono text-body text-text outline-none"
          value={draft.id}
          // wording-ok: a provider's own name, the same word in both languages
          placeholder="openai"
          onInput={(event) => { edit("id", event.currentTarget.value); }}
        />
        <span class="text-text-faint">{say("setup_id_help")}</span>
      </label>
      <label class="flex flex-col gap-tight text-note text-text-quiet">
        {say("setup_display_name")}
        <input
          class="rounded-control bg-g2 px-base py-snug text-body text-text outline-none"
          value={draft.label}
          onInput={(event) => { edit("label", event.currentTarget.value); }}
        />
        <span class="text-text-faint">{say("setup_not_carried")}</span>
      </label>
      <label class="flex flex-col gap-tight text-note text-text-quiet">
        {say("setup_base_url")}
        <input
          class="rounded-control bg-g2 px-base py-snug font-mono text-body text-text outline-none"
          value={draft.baseUrl}
          // wording-ok: an address, which no language translates
          placeholder="https://api.openai.com/v1"
          onInput={(event) => { edit("baseUrl", event.currentTarget.value); }}
        />
      </label>
      <div class="flex flex-col gap-tight text-note text-text-quiet">
        {say("setup_wire_api")}
        <div class="flex gap-snug">
          <For each={WIRE_APIS}>
            {(each) => (
              <button
                type="button"
                class={`rounded-pill px-base py-tight text-label ${draft.wireApi === each ? "bg-accent text-g0" : "bg-g2 text-text-quiet hover:bg-g3"}`}
                onClick={() => { setDraft("wireApi", each); }}
              >
                {wireWord(each)}
              </button>
            )}
          </For>
        </div>
        <Show when={dialect() === null}>
          <span class="text-alert">{say("setup_wire_api_unsupported")}</span>
        </Show>
      </div>
      <label class="flex flex-col gap-tight text-note text-text-quiet">
        {say("setup_key")}
        <input
          type="password"
          autocomplete="off"
          class="rounded-control bg-g2 px-base py-snug font-mono text-body text-text outline-none"
          value={draft.key}
          onInput={(event) => { edit("key", event.currentTarget.value); }}
        />
        <Show when={field().kind === "stored"}>
          <span class="text-text-faint">{say("setup_key_stored")}</span>
          <span class="font-mono text-text-disabled">{reference()}</span>
          <span class="flex gap-snug">
            <button
              type="button"
              class="rounded-control bg-g2 px-base py-tight text-label hover:bg-g3"
              onClick={() => {
                setHeld(null);
              }}
            >
              {say("setup_key_replace")}
            </button>
            <button
              type="button"
              class="rounded-control bg-g2 px-base py-tight text-label hover:bg-g3"
              onClick={() => {
                setHeld(null);
                setDraft("key", "");
              }}
            >
              {say("setup_key_clear")}
            </button>
          </span>
          <span class="text-text-faint">{say("setup_key_vault_note")}</span>
        </Show>
      </label>

      <div class="flex flex-col gap-snug">
        <button
          type="button"
          class="self-start rounded-control px-snug py-tight text-label text-text-quiet hover:bg-g2"
          aria-expanded={advanced()}
          onClick={() => setAdvanced(!advanced())}
        >
          {say("setup_advanced")}
        </button>
        <Show when={advanced()}>
          <div class="flex flex-col gap-snug rounded-card bg-g1 px-base py-snug">
            <div class="flex flex-wrap gap-snug">
              <Figure
                label={say("setup_timeout_ms")}
                value={draft.timeoutMs}
                onInput={(value) => { setDraft("timeoutMs", value); }}
              />
              <Figure
                label={say("setup_request_retries")}
                value={draft.requestRetries}
                onInput={(value) => { setDraft("requestRetries", value); }}
              />
              <Figure
                label={say("setup_stream_retries")}
                value={draft.streamRetries}
                onInput={(value) => { setDraft("streamRetries", value); }}
              />
              <Figure
                label={say("setup_stream_idle")}
                value={draft.streamIdleMs}
                onInput={(value) => { setDraft("streamIdleMs", value); }}
              />
            </div>
            <PairTable
              title={say("setup_headers")}
              nameLabel={say("setup_header_name")}
              valueLabel={say("setup_header_value")}
              rows={draft.headers}
              onChange={(next) => { setDraft("headers", [...next]); }}
            />
            <PairTable
              title={say("setup_overrides")}
              nameLabel={say("setup_pointer")}
              valueLabel={say("setup_override_value")}
              rows={draft.overrides}
              onChange={(next) => { setDraft("overrides", [...next]); }}
            />
            <p class="text-note text-text-faint">{say("setup_advanced_inert")}</p>
          </div>
        </Show>
      </div>

      <Show when={note()}>{(text) => <p class="text-note text-alert">{text()}</p>}</Show>
      <Show when={report()}>
        {(error) => <Reachability error={error()} host={hostOf(draft.baseUrl) ?? draft.baseUrl} />}
      </Show>
      <Show when={looking()}>
        <p class="text-note text-text-quiet">{say("setup_probe_busy", { host: hostOf(draft.baseUrl) ?? "" })}</p>
      </Show>

      <div class="flex flex-wrap items-center gap-snug">
        <button
          type="button"
          class="rounded-control bg-g2 px-base py-snug text-label hover:bg-g3 disabled:text-text-disabled"
          disabled={!complete() || busy()}
          onClick={() => {
            setReport(null);
            withKey((e) => {
              if (command(probeEndpoint(e))) setLooking(true);
            });
          }}
        >
          {say("setup_look")}
        </button>
        <button
          type="button"
          class="rounded-control bg-accent px-base py-snug text-label text-g0 hover:bg-accent-hover disabled:bg-g3 disabled:text-text-disabled"
          disabled={!complete() || busy()}
          onClick={() => {
            setReport(null);
            withKey((e) => {
              const chosen = rows();
              if (!command(attachEndpoint(e, chosen.map((row) => row.id)))) return;
              for (const row of chosen) {
                if (row.tag !== null) command(selectModel(e.id, row.id, row.tag, row.ceilings));
              }
              // The form starts over for the next provider. Only a
              // registration that went through clears it: a refusal
              // keeps every field, which is the one thing a person
              // filling in a key cannot be asked to do twice.
              setDraft({ ...FRESH });
              setHeld(null);
              setRows([]);
              props.onAttached?.();
            });
          }}
        >
          {say("setup_attach_btn")}
        </button>
      </div>

      <ModelTable served={models()} onChosen={setRows} />

      <details class="rounded-card bg-g1 px-base py-snug text-note">
        <summary class="cursor-pointer text-text-quiet">{say("setup_preview_toml")}</summary>
        <pre class="mt-snug overflow-auto whitespace-pre-wrap font-mono text-text-faint">
          {configToml(draft, reference())}
        </pre>
      </details>
      <details class="rounded-card bg-g1 px-base py-snug text-note">
        <summary class="cursor-pointer text-text-quiet">{say("setup_preview_request")}</summary>
        <pre class="mt-snug overflow-auto whitespace-pre-wrap font-mono text-text-faint">
          {requestPreview(draft, reference(), rows()[0]?.id ?? models()[0] ?? "<model>")}
        </pre>
      </details>
    </form>
  );
}

// The reference an id derives, shown before anything is enrolled so a
// person can see what the form is about to file their key under.
function referenceOf(id: string): string {
  const { realm, name } = referenceFor(id === "" ? "<id>" : id);
  return `secret:${realm}/${name}`;
}

// The three wire_api values, in Codex's own spelling. Not in the phrase
// table: each is one token that reads the same in both languages, and a
// translated wire value would be a value nobody can paste into a
// config.toml.
function wireWord(api: WireApi): string {
  // wording-ok: the literal values of Codex's `wire_api` key
  return api;
}

function useLoginUrl(provider: () => string) {
  const ui = useUi();
  const history = ui.conn.asking.ask({ history: { before: null, limit: 40 } });
  return createMemo<string | null>(() => {
    const answer = history();
    if (answer === undefined || !("history" in answer)) return null;
    for (let i = answer.history.records.length - 1; i >= 0; i -= 1) {
      const record = answer.history.records[i];
      if (record?.kind !== "login_started" || record.data.provider !== provider()) continue;
      const url = record.data.auth_url;
      return typeof url === "string" ? url : null;
    }
    return null;
  });
}

export function LoginForm(props: { readonly onAttached?: () => void }) {
  const say = useSay();
  const command = useCommand();
  const [provider] = createSignal("anthropic");
  const [code, setCode] = createSignal("");
  const url = useLoginUrl(provider);
  return (
    <div class="flex flex-col gap-base">
      <div class="flex items-center gap-snug">
        <button
          type="button"
          class="rounded-control bg-g2 px-base py-snug text-label hover:bg-g3"
          onClick={() => command(loginBegin(provider()))}
        >
          {say("setup_login_begin")} · {provider()}
        </button>
        <Show when={url()}>
          {(href) => (
            <a href={href()} target="_blank" rel="noreferrer" class="text-note text-accent underline">
              {say("setup_login_open")}
            </a>
          )}
        </Show>
      </div>
      <Show when={url()}>
        <label class="flex flex-col gap-tight text-note text-text-quiet">
          {say("setup_login_code")}
          <div class="flex gap-snug">
            <input
              class="flex-1 rounded-control bg-g2 px-base py-snug font-mono text-body text-text outline-none"
              value={code()}
              onInput={(event) => setCode(event.currentTarget.value)}
            />
            <button
              type="button"
              class="rounded-control bg-accent px-base py-snug text-label text-g0 hover:bg-accent-hover disabled:bg-g3 disabled:text-text-disabled"
              disabled={code().trim() === ""}
              onClick={() => {
                if (command(loginCode(provider(), code().trim()))) {
                  setCode("");
                  props.onAttached?.();
                }
              }}
            >
              {say("setup_login_finish")}
            </button>
          </div>
        </label>
      </Show>
    </div>
  );
}

export function EndpointList(props: { readonly answer: EndpointsAnswer }) {
  const say = useSay();
  return (
    <ul class="flex flex-col gap-snug">
      <For each={props.answer.endpoints}>
        {(endpoint) => (
          <li class="rounded-card bg-g1 px-base py-snug text-note">
            <div class="flex items-center gap-snug">
              <span class="font-label text-text">{endpoint.name}</span>
              <span class="text-text-faint">{wireWord(endpoint.dialect === "anthropic" ? "messages" : "chat")}</span>
              <span class="flex-1 truncate font-mono text-text-disabled">{endpoint.base_url}</span>
              <span class="text-text-faint">{endpoint.has_credential ? say("setup_keyed") : say("setup_unkeyed")}</span>
            </div>
            <div class="mt-tight flex flex-wrap gap-tight text-text-quiet">
              <For each={endpoint.models}>{(model) => <span class="rounded-pill bg-g2 px-snug">{model}</span>}</For>
            </div>
          </li>
        )}
      </For>
    </ul>
  );
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Turning a URL and a key into models a run can be given. The key goes
// to the vault by its own route and comes back as a reference; the
// reference is what the attach frame carries. A subscription login is
// the other door: begin, approve in a browser, bring back the code.

import { For, Show, createMemo, createSignal } from "solid-js";

import { attachEndpoint, loginBegin, loginCode, probeEndpoint } from "../../core/commands";
import type { Endpoint } from "../../core/commands";
import { enrol, referenceFor } from "../../core/enrol";
import type { Enrolment } from "../../core/enrol";
import type { DialectKind, EndpointsAnswer } from "../../wire";
import { useCommand, useSay, useUi } from "../../ui";

// The probe's answer arrives as a record; the page reads it off the
// tail of the history rather than folding it, because it is one fact
// wanted once.
function useProbed() {
  const ui = useUi();
  const history = ui.conn.asking.ask({ history: { before: null, limit: 40 } });
  return createMemo<{ name: string; models: string[] } | null>(() => {
    const answer = history();
    if (answer === undefined || !("history" in answer)) return null;
    for (let i = answer.history.records.length - 1; i >= 0; i -= 1) {
      const record = answer.history.records[i];
      if (record?.kind !== "endpoint_probed") continue;
      const name = record.data.name;
      const models = record.data.models;
      if (typeof name === "string" && Array.isArray(models)) {
        return { name, models: models.filter((m): m is string => typeof m === "string") };
      }
    }
    return null;
  });
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

export function AttachForm(props: { readonly onAttached?: () => void }) {
  const ui = useUi();
  const say = useSay();
  const command = useCommand();
  const [name, setName] = createSignal("");
  const [baseUrl, setBaseUrl] = createSignal("");
  const [dialect, setDialect] = createSignal<DialectKind>("open_ai");
  const [key, setKey] = createSignal("");
  const [reference, setReference] = createSignal<string | null>(null);
  const [note, setNote] = createSignal<string | null>(null);
  const [busy, setBusy] = createSignal(false);
  const probed = useProbed();
  const models = createMemo(() => {
    const held = probed();
    return held !== null && held.name === name().trim() ? held.models : [];
  });

  const endpoint = (): Endpoint => ({
    name: name().trim(),
    baseUrl: baseUrl().trim(),
    dialect: dialect(),
    secret: reference(),
    authHeader: null,
  });
  // The key is enrolled once, on the first action that needs it; the
  // reference is kept for the second.
  const withKey = (then: (e: Endpoint) => void) => {
    if (reference() !== null || key().trim() === "") {
      then(endpoint());
      return;
    }
    setBusy(true);
    const { realm, name: keyName } = referenceFor(name().trim());
    // The form as it stands when the key leaves; what comes back is
    // attached to that, not to whatever the fields say later.
    const staticAsked = endpoint();
    const settle = (outcome: Enrolment) => {
      setBusy(false);
      if (outcome.kind === "stored") {
        setReference(outcome.reference);
        setKey("");
        then({ ...staticAsked, secret: outcome.reference });
      } else {
        setNote(outcome.reason);
      }
    };
    void enrol(ui.origin, realm, keyName, key().trim()).then(settle);
  };
  const complete = () => name().trim() !== "" && /^https?:\/\//.test(baseUrl().trim());

  return (
    <form
      class="flex flex-col gap-base"
      onSubmit={(event) => {
        event.preventDefault();
      }}
    >
      <label class="flex flex-col gap-tight text-note text-text-quiet">
        {say("setup_name")}
        <input
          class="rounded-control bg-g2 px-base py-snug text-body text-text outline-none"
          value={name()}
          placeholder="openai"
          onInput={(event) => setName(event.currentTarget.value)}
        />
      </label>
      <label class="flex flex-col gap-tight text-note text-text-quiet">
        {say("setup_base_url")}
        <input
          class="rounded-control bg-g2 px-base py-snug font-mono text-body text-text outline-none"
          value={baseUrl()}
          placeholder="https://api.openai.com/v1"
          onInput={(event) => setBaseUrl(event.currentTarget.value)}
        />
      </label>
      <div class="flex flex-col gap-tight text-note text-text-quiet">
        {say("setup_dialect")}
        <div class="flex gap-snug">
          <For each={["open_ai", "anthropic"] satisfies DialectKind[]}>
            {(each) => (
              <button
                type="button"
                class={`rounded-pill px-base py-tight text-label ${dialect() === each ? "bg-accent text-g0" : "bg-g2 text-text-quiet hover:bg-g3"}`}
                onClick={() => setDialect(each)}
              >
                {say(`dialect_${each}`)}
              </button>
            )}
          </For>
        </div>
      </div>
      <label class="flex flex-col gap-tight text-note text-text-quiet">
        {say("setup_key")}
        <Show when={reference() === null} fallback={<span class="font-mono text-text-faint">{reference()}</span>}>
          <input
            type="password"
            autocomplete="off"
            class="rounded-control bg-g2 px-base py-snug font-mono text-body text-text outline-none"
            value={key()}
            onInput={(event) => setKey(event.currentTarget.value)}
          />
        </Show>
      </label>
      <Show when={note()}>{(text) => <p class="text-note text-alert">{text()}</p>}</Show>
      <div class="flex flex-wrap items-center gap-snug">
        <button
          type="button"
          class="rounded-control bg-g2 px-base py-snug text-label hover:bg-g3 disabled:text-text-disabled"
          disabled={!complete() || busy()}
          onClick={() => {
            withKey((e) => {
              command(probeEndpoint(e));
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
            withKey((e) => {
              if (command(attachEndpoint(e, []))) props.onAttached?.();
            });
          }}
        >
          {say("setup_attach_btn")}
        </button>
      </div>
      <Show when={models().length > 0}>
        <ul class="flex flex-wrap gap-tight text-note text-text-quiet">
          <For each={models()}>{(model) => <li class="rounded-pill bg-g2 px-snug">{model}</li>}</For>
        </ul>
      </Show>
    </form>
  );
}

export function LoginForm(props: { readonly onAttached?: () => void }) {
  const say = useSay();
  const command = useCommand();
  const [provider] = createSignal("anthropic");
  const [code, setCode] = createSignal("");
  const url = useLoginUrl(provider);
  return (
    <div class="flex flex-col gap-base">
      <p class="text-note text-text-quiet">{say("setup_login_body")}</p>
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
              <span class="text-text-faint">{say(`dialect_${endpoint.dialect}`)}</span>
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

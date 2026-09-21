// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The two doors a provider is reached through, and what came in through
// them. The key door is `providers/form.tsx` and the draft it edits is
// `providers/draft.tsx`; here are the subscription door and the list of
// what is attached.
//
// A subscription login is the other door: begin, approve in a browser,
// bring back the code.

import { For, Show, createMemo, createSignal } from "solid-js";

import { loginBegin, loginCode, wireApiOf } from "../../core/commands";
import type { EndpointsAnswer } from "../../wire";
import { useCommand, useSay, useUi } from "../../ui";
import { Badge } from "../parts/badge";
import { EmptyState } from "../parts/empty";
import { Field } from "../parts/field";

export { AttachForm } from "./providers/form";
export { PROXYINGS, proxyingNote } from "./providers/draft";

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
        <div class="flex flex-col gap-tight text-note text-text-quiet">
          <div class="flex items-end gap-snug">
            <Field
              label={say("setup_login_code")}
              mono
              value={code()}
              onInput={setCode}
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
        </div>
      </Show>
    </div>
  );
}

// What is attached, one row each.
//
// **The row is headed by the name the person gave it.** `label` is the
// display name, and the city answers with the id when nobody stated
// one, so this row never has to decide what to show. The id appears
// beside it only when the two differ, because that is when a person
// needs both: the label to recognise the provider, and the id to find
// its table in a `config.toml` and its key in the vault.
//
// **The badge says the call never leaves this machine.** The city
// decides that from the base URL and states it as `local`, and a
// confidential building is refused every endpoint without it
// (`gateway::router::book::select`), so it is the one property of a
// row that changes what a person may do with it.
export function EndpointList(props: { readonly answer: EndpointsAnswer }) {
  const say = useSay();
  return (
    <Show
      when={props.answer.endpoints.length > 0}
      fallback={<EmptyState text={say("setup_no_endpoints")} />}
    >
      <ul class="flex flex-col gap-snug">
        <For each={props.answer.endpoints}>
          {(endpoint) => (
            <li class="rounded-card bg-g1 px-base py-snug text-note">
              <div class="flex items-center gap-snug">
                <span class="font-label text-text">{endpoint.label}</span>
                <Show when={endpoint.label !== endpoint.name}>
                  <span class="font-mono text-text-faint">{endpoint.name}</span>
                </Show>
                <Show when={endpoint.local}>
                  <Badge text={say("setup_local")} />
                </Show>
                <span class="text-text-faint">{wireApiOf(endpoint.dialect)}</span>
                <span class="flex-1 truncate font-mono text-text-disabled">{endpoint.base_url}</span>
                <span class="text-text-faint">{endpoint.has_credential ? say("setup_keyed") : say("setup_unkeyed")}</span>
              </div>
              <div class="mt-tight flex flex-wrap gap-tight text-text-quiet">
                <For each={endpoint.models}>
                  {(row) => <span class="rounded-pill bg-g2 px-snug">{row.id}</span>}
                </For>
              </div>
            </li>
          )}
        </For>
      </ul>
    </Show>
  );
}

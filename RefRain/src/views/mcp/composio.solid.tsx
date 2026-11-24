// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Match, Show, Switch, createEffect, createMemo, createSignal } from "solid-js";
import { QUERIES } from "../../core/asking";
import { enrol } from "../../core/enrol";
import { say, type Lang } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { ToolkitLine, ToolkitSlug } from "../../wire";
import Badge from "../parts/badge.solid";
import Button from "../parts/button.solid";
import Field from "../parts/field.solid";
import { recoveryWords } from "../parts/notice_title";
import Unanswered from "../parts/unanswered.solid";
import { EMPTY, WHY, encode, type Intake } from "./draft";
import { consentUrl, shelfOf } from "./shelf";

const COMPOSIO_BASE = "https://backend.composio.dev/v3/mcp/";
const KEY_HEADER = "x-api-key";

function Toolkit(props: {
  readonly line: ToolkitLine;
  readonly lang: Lang;
  readonly connect: (line: ToolkitLine) => void;
  readonly recheck: () => void;
}) {
  const connected = () => {
    const state = props.line.standing;
    return state !== "absent" && "connected" in state ? state.connected : undefined;
  };
  const refused = () => {
    const state = props.line.standing;
    return state !== "absent" && "refused" in state ? state.refused.refusal : undefined;
  };
  return (
    <li class="rounded-card bg-chrome">
      <div class="flex min-w-0 items-center gap-base px-base py-snug text-note">
        <span class="w-figure shrink-0 truncate text-text">{props.line.name}</span>
        <span class="min-w-0 flex-1 truncate font-mono text-text-faint">{props.line.slug}</span>
        <Badge text={props.line.auth} />
        <Switch>
          <Match when={props.line.standing === "absent"}>
            <Button
              label={say(props.lang, "mcp_connect")}
              tone="secondary"
              onPress={() => {
                props.connect(props.line);
              }}
            />
          </Match>
          <Match when={connected()}>
            {(held) => <Badge text={held().alias} weight="live" dot />}
          </Match>
          <Match when={consentUrl(props.line)}>
            {(url) => (
              <a
                href={url()}
                target="_blank"
                rel="noopener noreferrer"
                class="text-note text-accent hover:underline"
                onClick={props.recheck}
              >
                {say(props.lang, "mcp_consent_finish")}
              </a>
            )}
          </Match>
          <Match when={refused()}>
            {(error) => (
              <div class="flex min-w-0 items-center gap-snug">
                <span class="min-w-0 truncate text-note text-alert">
                  {recoveryWords(props.lang, error().code, error().recovery)}
                </span>
                <Button
                  label={say(props.lang, "mcp_connect_again")}
                  tone="secondary"
                  onPress={() => {
                    props.connect(props.line);
                  }}
                />
              </div>
            )}
          </Match>
        </Switch>
      </div>
    </li>
  );
}

export default function Composio(props: { readonly intake: Intake }) {
  const u = ui();
  const lang = storeValue(u.lang);
  const shelf = shelfOf();
  const read = storeValue(shelf.state);
  const [key, setKey] = createSignal("");
  const [reference, setReference] = createSignal<string>();
  const [refused, setRefused] = createSignal(false);
  const [saving, setSaving] = createSignal(false);
  const [pressed, setPressed] = createSignal<ToolkitSlug>();
  const [serverId, setServerId] = createSignal("");
  const [userId, setUserId] = createSignal("");
  let tab: Window | null = null;
  let routed: ToolkitSlug | undefined;
  const answer = createMemo(() => {
    const item = read();
    return item.kind === "held" ? item.value : undefined;
  });
  const missing = createMemo(() => {
    const item = read();
    return item.kind === "unavailable" ? item.query : undefined;
  });
  const status = createMemo(() => {
    const item = answer();
    return item !== undefined && item !== "unenrolled" && "refused" in item
      ? item.refused.refusal
      : undefined;
  });
  const rows = createMemo(() => shelf.rows(read()));
  createEffect(
    () => [pressed(), rows()] as const,
    ([waiting, lines]) => {
      if (waiting === undefined || routed === waiting) return;
      const row = lines.find((line) => line.slug === waiting);
      const url = row === undefined ? null : consentUrl(row);
      if (url === null) return;
      if (tab !== null && !tab.closed) tab.location.href = url;
      routed = waiting;
    },
  );
  async function store(): Promise<void> {
    const value = key().trim();
    if (value === "" || saving()) return;
    setSaving(true);
    setRefused(false);
    const outcome = await enrol({
      origin: u.origin,
      token: u.credential(),
      realm: "mcp",
      name: "composio",
      value,
      lang: lang(),
    });
    setSaving(false);
    if (outcome.kind !== "stored") {
      setRefused(true);
      return;
    }
    setReference(outcome.reference);
    setKey("");
    shelf.recheck();
  }
  function connect(line: ToolkitLine): void {
    routed = undefined;
    tab = window.open("", "_blank", "noopener");
    setPressed(line.slug);
    shelf.connect(line.slug);
  }
  const encoded = createMemo(() => {
    const id = serverId().trim();
    const user = userId().trim();
    const credential = reference();
    return encode(
      {
        ...EMPTY,
        label: id,
        transport: "http",
        url:
          id === ""
            ? ""
            : `${COMPOSIO_BASE}${id}${user === "" ? "" : `?user_id=${encodeURIComponent(user)}`}`,
        headers: credential === undefined ? [] : [{ name: KEY_HEADER, value: credential }],
      },
      props.intake.taken(),
    );
  });
  const reason = createMemo(() => {
    const scope = props.intake.why();
    if (scope !== null) return scope;
    if (reference() === undefined) return say(lang(), "mcp_key_first");
    const item = encoded();
    return item.kind === "blocked" ? say(lang(), WHY[item.blocker]) : undefined;
  });
  function send(): void {
    const item = encoded();
    if (item.kind === "ready" && props.intake.offer(item.server)) {
      setServerId("");
      setUserId("");
    }
  }
  return (
    <div class="flex flex-col gap-base">
      <div class="flex flex-wrap items-end gap-base">
        <div class="min-w-0 flex-[1_1_16ch]">
          <Field
            label={say(lang(), "mcp_api_key")}
            kind="password"
            mono
            value={key()}
            onInput={setKey}
          />
        </div>
        <Button
          label={say(lang(), "mcp_key_store")}
          loading={saving()}
          tone="secondary"
          onPress={() => {
            void store();
          }}
        />
      </div>
      <a
        href="https://platform.composio.dev"
        target="_blank"
        rel="noopener noreferrer"
        class="w-fit text-note text-text-faint transition-colors ease-leave hover:text-text-quiet hover:ease-arrive"
      >
        {say(lang(), "mcp_open_composio")}
      </a>
      <Show when={reference()}>
        {(held) => (
          <div class="flex min-w-0 items-center gap-snug text-note" role="status">
            <Badge text={say(lang(), "mcp_key_stored")} weight="live" dot />
            <span class="min-w-0 truncate font-mono text-text-faint">{held()}</span>
          </div>
        )}
      </Show>
      <Show when={refused()}>
        <p class="text-note text-alert" role="alert">
          {say(lang(), "link_refused")}
        </p>
      </Show>
      <Switch>
        <Match when={missing()}>
          {(query) => <Unanswered query={query()} asked={QUERIES.toolkits} />}
        </Match>
        <Match when={answer() === undefined}>
          <p class="text-note text-text-faint">{say(lang(), "mcp_shelf_asking")}</p>
        </Match>
        <Match when={answer() === "unenrolled"}>
          <p class="text-note text-text-faint">{say(lang(), "mcp_shelf_needs_key")}</p>
        </Match>
        <Match when={status()}>
          {(error) => (
            <div class="flex flex-col gap-tight" role="alert">
              <p class="text-note text-alert">{error().subject}</p>
              <p class="text-note text-text-faint">
                {recoveryWords(lang(), error().code, error().recovery)}
              </p>
              <Button label={say(lang(), "mcp_shelf_again")} tone="quiet" onPress={shelf.recheck} />
            </div>
          )}
        </Match>
        <Match when={true}>
          <ul class="flex flex-col gap-tight" aria-label={say(lang(), "mcp_toolkits")}>
            <For each={rows()} keyed={(item) => item.slug}>
              {(item) => (
                <Toolkit line={item()} lang={lang()} connect={connect} recheck={shelf.recheck} />
              )}
            </For>
          </ul>
        </Match>
      </Switch>
      <details class="text-note">
        <summary class="cursor-pointer text-text-faint transition-colors ease-leave hover:text-text-quiet hover:ease-arrive">
          {say(lang(), "mcp_by_id")}
        </summary>
        <div class="mt-base flex flex-col gap-base">
          <div class="grid gap-base grid-cols-[repeat(auto-fit,minmax(320px,1fr))]">
            <Field
              label={say(lang(), "mcp_server_id")}
              mono
              value={serverId()}
              onInput={setServerId}
            />
            <Field label={say(lang(), "mcp_user_id")} mono value={userId()} onInput={setUserId} />
          </div>
          <Show
            when={reason()}
            fallback={<Button label={say(lang(), "mcp_add")} tone="secondary" onPress={send} />}
          >
            {(why) => (
              <div class="flex min-w-0 items-center gap-base">
                <Button label={say(lang(), "mcp_add")} why={why()} />
                <span class="min-w-0 text-note text-text-faint">{why()}</span>
              </div>
            )}
          </Show>
        </div>
      </details>
    </div>
  );
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One key, then one press per application.
//
// The key goes to the vault over the enrolment door and what comes back
// is a reference, so the key itself is never in a frame and never in
// `CONFIG.toml`. Everything after that is the broker's to answer: the
// directory, which account is connected, and the consent page to send
// somebody to.
//
// **Pressing connect opens a tab in the same gesture**, before the city
// has answered with the page to put in it. A browser blocks a popup
// opened after a round trip, and a person who pressed a button and got
// nothing has no way to tell a blocked popup from a broken city; the
// blank tab is opened while the click is still on the stack and given
// its address when the answer lands.

import { For, Match, Show, Switch, createEffect, createSignal } from "solid-js";

import { EMPTY, WHY, encode } from "./draft";
import type { Intake } from "./draft";
import type { ToolkitLine, ToolkitSlug } from "../../wire";
import { consentUrl, shelfOf } from "./shelf";
import { Badge } from "../parts/badge";
import { Button } from "../parts/button";
import { Field } from "../parts/field";
import { enrol } from "../../core/enrol";
import { useSay, useUi } from "../../ui";

// Where a Composio-hosted server answers, for somebody who made one in
// the broker's own console and wants to point this city at it by id.
// The path every connected application takes does not pass through
// here.
const COMPOSIO_BASE = "https://backend.composio.dev/v3/mcp/";

// The header the broker reads the key from.
const KEY_HEADER = "x-api-key";

export function Composio(props: { readonly intake: Intake }) {
  const ui = useUi();
  const say = useSay();
  const shelf = shelfOf();
  const [key, setKey] = createSignal("");
  const [reference, setReference] = createSignal<string | null>(null);
  const [refused, setRefused] = createSignal(false);
  const [pressed, setPressed] = createSignal<ToolkitSlug | null>(null);
  const [tab, setTab] = createSignal<Window | null>(null);

  const store = () => {
    const value = key().trim();
    if (value === "") return;
    setRefused(false);
    void enrol(ui.origin, "mcp", "composio", value).then((outcome) => {
      if (outcome.kind === "stored") {
        setReference(outcome.reference);
        setKey("");
        // The key is what the shelf could not be read without, so the
        // directory is asked for the moment there is one.
        shelf.recheck();
        return;
      }
      setRefused(true);
    });
  };

  // The tab opened by the press, given its address as soon as the city
  // says where the consent page is. A person who dismissed the tab gets
  // the link on the row instead, which is why the url keeps travelling.
  createEffect(() => {
    const waiting = pressed();
    if (waiting === null) return;
    const row = shelf.rows().find((line) => line.slug === waiting);
    const url = row === undefined ? null : consentUrl(row);
    if (url === null) return;
    const held = tab();
    if (held !== null && !held.closed) held.location.href = url;
    setPressed(null);
    setTab(null);
  });

  const press = (line: ToolkitLine) => {
    setTab(window.open("", "_blank", "noopener"));
    setPressed(line.slug);
    shelf.connect(line.slug);
  };

  return (
    <div class="flex flex-col gap-base">
      <div class="flex flex-wrap items-end gap-base">
        <div class="min-w-0 flex-1">
          <Field
            label={say("mcp_api_key")}
            kind="password"
            mono
            value={key()}
            onInput={(typed) => {
              setKey(typed);
            }}
          />
        </div>
        <Button label={say("mcp_key_store")} tone="secondary" onPress={store} />
        <a
          href="https://platform.composio.dev"
          target="_blank"
          rel="noreferrer"
          class="text-note text-text-faint hover:text-text-quiet"
        >
          {say("mcp_open_composio")}
        </a>
      </div>
      <Show when={reference()}>
        {(held) => (
          <div class="flex min-w-0 items-center gap-snug text-note">
            <Badge text={say("mcp_key_stored")} weight="live" dot />
            <span class="min-w-0 truncate font-mono text-text-faint">{held()}</span>
          </div>
        )}
      </Show>
      <Show when={refused()}>
        <p class="text-note text-alert" role="alert">
          {say("link_refused")}
        </p>
      </Show>

      <Shelf shelf={shelf} press={press} intake={props.intake} />

      <details class="text-note">
        <summary class="cursor-pointer text-text-faint hover:text-text-quiet">
          {say("mcp_by_id")}
        </summary>
        <div class="mt-base">
          <ByBrokerId intake={props.intake} reference={reference()} />
        </div>
      </details>
    </div>
  );
}

/// The directory, or the one sentence that stands in for it.
function Shelf(props: {
  readonly shelf: ReturnType<typeof shelfOf>;
  readonly press: (line: ToolkitLine) => void;
  readonly intake: Intake;
}) {
  const say = useSay();
  const held = () => props.shelf.answer();
  return (
    <Show when={held()} fallback={<p class="text-note text-text-faint">{say("mcp_shelf_asking")}</p>}>
      {(answer) => {
        const value = answer();
        if (value === "unenrolled" || !("shelf" in value || "refused" in value)) {
          return <p class="text-note text-text-faint">{say("mcp_shelf_needs_key")}</p>;
        }
        if ("refused" in value) {
          return (
            <div class="flex flex-col gap-tight" role="alert">
              <p class="text-note text-alert">{value.refused.refusal.subject}</p>
              <p class="text-note text-text-faint">{value.refused.refusal.recovery}</p>
              <Button label={say("mcp_shelf_again")} tone="quiet" onPress={props.shelf.recheck} />
            </div>
          );
        }
        return (
          <ul class="flex flex-col gap-tight" aria-label={say("mcp_toolkits")}>
            <For each={value.shelf.toolkits}>
              {(line) => <Row line={line} press={props.press} recheck={props.shelf.recheck} />}
            </For>
          </ul>
        );
      }}
    </Show>
  );
}

/// One application: what it is, and the one thing to do about it.
function Row(props: {
  readonly line: ToolkitLine;
  readonly press: (line: ToolkitLine) => void;
  readonly recheck: () => void;
}) {
  const standing = () => props.line.standing;
  return (
    <li class="rounded-card bg-g1">
      <div class="flex min-w-0 items-center gap-base px-base py-snug text-note">
        <span class="w-figure shrink-0 truncate text-text">{props.line.name}</span>
        <span class="min-w-0 flex-1 truncate font-mono text-text-faint">{props.line.slug}</span>
        <Badge text={props.line.auth} />
        <Action line={props.line} press={props.press} recheck={props.recheck} standing={standing()} />
      </div>
    </li>
  );
}

/// Four standings, four different next actions and no fifth.
function Action(props: {
  readonly line: ToolkitLine;
  readonly standing: ToolkitLine["standing"];
  readonly press: (line: ToolkitLine) => void;
  readonly recheck: () => void;
}) {
  const say = useSay();
  const held = () => props.standing;
  const connected = () => {
    const standing = held();
    return standing !== "absent" && "connected" in standing ? standing.connected : null;
  };
  const awaiting = () => {
    const standing = held();
    return standing !== "absent" && "awaiting" in standing ? standing.awaiting : null;
  };
  const refused = () => {
    const standing = held();
    return standing !== "absent" && "refused" in standing ? standing.refused : null;
  };
  const again = (label: string) => (
    <Button
      label={label}
      tone="primary"
      onPress={() => {
        props.press(props.line);
      }}
    />
  );
  return (
    <Switch fallback={again(say("mcp_connect"))}>
      <Match when={connected()}>
        {(value) => <Badge text={value().alias} weight="live" dot />}
      </Match>
      <Match when={awaiting()}>
        {(value) => (
          <a
            href={value().consent_url}
            target="_blank"
            rel="noreferrer"
            class="text-note text-accent hover:underline"
            onClick={() => {
              // Returning to this window re-reads the shelf, so a person
              // who finishes here sees the row settle without pressing
              // anything else.
              props.recheck();
            }}
          >
            {say("mcp_consent_finish")}
          </a>
        )}
      </Match>
      <Match when={refused()}>
        {(value) => (
          <div class="flex min-w-0 items-center gap-snug">
            <span class="min-w-0 truncate text-note text-alert">
              {value().refusal.recovery}
            </span>
            {again(say("mcp_connect_again"))}
          </div>
        )}
      </Match>
    </Switch>
  );
}

/// For somebody who already made a server in the broker's own console.
/// Kept because it costs one collapsed section and answers the one case
/// the directory cannot: a server this city did not open.
function ByBrokerId(props: { readonly intake: Intake; readonly reference: string | null }) {
  const say = useSay();
  const [serverId, setServerId] = createSignal("");
  const [userId, setUserId] = createSignal("");
  const url = () => {
    const user = userId().trim();
    const id = serverId().trim();
    return `${COMPOSIO_BASE}${id}${user === "" ? "" : `?user_id=${encodeURIComponent(user)}`}`;
  };
  const encoded = () =>
    encode(
      {
        ...EMPTY,
        label: serverId().trim(),
        transport: "http",
        url: serverId().trim() === "" ? "" : url(),
        headers:
          props.reference === null ? [] : [{ name: KEY_HEADER, value: props.reference }],
      },
      props.intake.taken(),
    );
  const reason = () => {
    const scope = props.intake.why();
    if (scope !== null) return scope;
    if (props.reference === null) return say("mcp_key_first");
    const out = encoded();
    return out.kind === "blocked" ? say(WHY[out.blocker]) : null;
  };
  return (
    <div class="flex flex-col gap-base">
      <div class="grid gap-base md:grid-cols-2">
        <Field
          label={say("mcp_server_id")}
          mono
          value={serverId()}
          onInput={(typed) => {
            setServerId(typed);
          }}
        />
        <Field
          label={say("mcp_user_id")}
          mono
          value={userId()}
          onInput={(typed) => {
            setUserId(typed);
          }}
        />
      </div>
      <Show
        when={reason()}
        fallback={
          <Button
            label={say("mcp_add")}
            tone="primary"
            onPress={() => {
              const out = encoded();
              if (out.kind === "ready" && props.intake.offer(out.server)) {
                setServerId("");
                setUserId("");
              }
            }}
          />
        }
      >
        {(why) => (
          <div class="flex min-w-0 items-center gap-base">
            <Button label={say("mcp_add")} why={why()} />
            <span class="min-w-0 text-note text-text-faint">{why()}</span>
          </div>
        )}
      </Show>
    </div>
  );
}

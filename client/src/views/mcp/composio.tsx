// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Composio as a directory rather than as three identifiers: one api key
// for the whole city, then a row per toolkit with the button that would
// connect it.
//
// The key is the only part of this that is finished. It goes to the
// vault over the enrolment door and what comes back is the reference
// every server built here carries in its header, so the key itself is
// never in a frame and never in `CONFIG.toml`.
//
// Connecting is inert, and says so on the row: the round trip opens a
// browser on the city's machine and waits for Composio to call back,
// and no command on the wire asks for that. Until it exists, a person
// who has already made a server in Composio's own console can still
// point this city at it by its id, and the header will carry the
// reference rather than the key.

import { For, Show, createSignal } from "solid-js";

import { EMPTY, WHY, encode } from "./draft";
import type { Intake } from "./draft";
import { TOOLKITS } from "./toolkits";
import type { Toolkit } from "./toolkits";
import { Badge } from "../parts/badge";
import { Button } from "../parts/button";
import { Field } from "../parts/field";
import { enrol } from "../../core/enrol";
import { useSay, useUi } from "../../ui";

// Where a Composio-hosted server answers. The id a person pastes is the
// last segment of it.
const COMPOSIO_BASE = "https://backend.composio.dev/v3/mcp/";

// The header Composio reads the key from.
const KEY_HEADER = "x-api-key";

export function Composio(props: { readonly intake: Intake }) {
  const ui = useUi();
  const say = useSay();
  const [key, setKey] = createSignal("");
  const [reference, setReference] = createSignal<string | null>(null);
  const [refused, setRefused] = createSignal(false);
  const [open, setOpen] = createSignal<string | null>(null);
  const [serverId, setServerId] = createSignal("");
  const [userId, setUserId] = createSignal("");

  const store = () => {
    const value = key().trim();
    if (value === "") return;
    setRefused(false);
    void enrol(ui.origin, "mcp", "composio", value).then((outcome) => {
      if (outcome.kind === "stored") {
        setReference(outcome.reference);
        setKey("");
        return;
      }
      setRefused(true);
    });
  };

  const url = () => {
    const user = userId().trim();
    const id = serverId().trim();
    return `${COMPOSIO_BASE}${id}${user === "" ? "" : `?user_id=${encodeURIComponent(user)}`}`;
  };
  const encoded = (toolkit: Toolkit) => {
    const held = reference();
    return encode(
      {
        ...EMPTY,
        label: toolkit.slug,
        transport: "http",
        url: serverId().trim() === "" ? "" : url(),
        headers: held === null ? [] : [{ name: KEY_HEADER, value: held }],
      },
      props.intake.taken(),
    );
  };
  const reason = (toolkit: Toolkit) => {
    const scope = props.intake.why();
    if (scope !== null) return scope;
    if (reference() === null) return say("mcp_key_first");
    const out = encoded(toolkit);
    return out.kind === "blocked" ? say(WHY[out.blocker]) : null;
  };
  const send = (toolkit: Toolkit) => {
    const out = encoded(toolkit);
    if (out.kind === "ready" && props.intake.offer(out.server)) {
      setServerId("");
      setUserId("");
      setOpen(null);
    }
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

      <ul class="flex flex-col gap-tight" aria-label={say("mcp_toolkits")}>
        <For each={TOOLKITS}>
          {(toolkit) => (
            <li class="rounded-card bg-g1">
              <div class="flex min-w-0 items-center gap-base px-base py-snug text-note">
                <span class="w-figure shrink-0 truncate text-text">{toolkit.name}</span>
                <span class="min-w-0 flex-1 truncate font-mono text-text-faint">{toolkit.slug}</span>
                <Badge text={toolkit.auth === "oauth" ? say("mcp_auth_oauth") : say("mcp_api_key")} />
                <Button label={say("mcp_connect")} why={say("mcp_gap_connect")} />
                <Button
                  label={say("mcp_by_id")}
                  tone="quiet"
                  onPress={() => {
                    setOpen(open() === toolkit.slug ? null : toolkit.slug);
                  }}
                />
              </div>
              <Show when={open() === toolkit.slug}>
                <div class="flex flex-col gap-base border-t border-g2 px-base py-snug">
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
                    when={reason(toolkit)}
                    fallback={
                      <Button
                        label={say("mcp_add")}
                        tone="primary"
                        onPress={() => {
                          send(toolkit);
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
              </Show>
            </li>
          )}
        </For>
      </ul>
      <p class="text-note text-text-faint">{say("mcp_gap_connect")}</p>
    </div>
  );
}

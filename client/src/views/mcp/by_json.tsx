// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// `claude mcp add-json`, as a form: paste the block another tool wrote
// and read, before anything is sent, which of its servers this city can
// be told about and why the rest cannot.
//
// The block is shown row by row rather than accepted whole: a settings
// file usually holds several servers, and one of them carrying an
// environment variable must not silently take the other three with it.

import { For, Show, createSignal } from "solid-js";

import { SPELLED, WHY, encode } from "./draft";
import type { Draft, Encoded, Intake } from "./draft";
import { Badge } from "../parts/badge";
import { Button } from "../parts/button";
import { readFragment } from "./fragment";
import { useSay } from "../../ui";

interface Read {
  readonly draft: Draft;
  readonly encoded: Encoded;
}

export function ByJson(props: { readonly intake: Intake }) {
  const say = useSay();
  const [text, setText] = createSignal("");

  const fragment = () => readFragment(text());
  const rows = (): readonly Read[] => {
    const read = fragment();
    if (read.kind === "unreadable") return [];
    return read.drafts.map((draft) => ({ draft, encoded: encode(draft, props.intake.taken()) }));
  };
  const ready = () => rows().filter((row) => row.encoded.kind === "ready");
  const reason = () => {
    const scope = props.intake.why();
    if (scope !== null) return scope;
    return ready().length === 0 ? say("mcp_json_empty") : null;
  };
  const send = () => {
    let every = true;
    for (const row of rows()) {
      if (row.encoded.kind === "ready") {
        every = props.intake.offer(row.encoded.server) && every;
      } else {
        every = false;
      }
    }
    if (every) {
      setText("");
    }
  };

  return (
    <div class="flex flex-col gap-base">
      <textarea
        class="min-h-output w-full min-w-0 rounded-control border border-g3 bg-g2 px-base py-snug font-mono text-note text-text outline-none placeholder:text-text-disabled"
        rows={8}
        aria-label={say("mcp_door_json")}
        placeholder={say("mcp_json_placeholder")}
        value={text()}
        onInput={(event) => {
          setText(event.currentTarget.value);
        }}
      />
      <Show when={text().trim() !== "" && fragment().kind === "unreadable"}>
        <p class="text-note text-alert" role="alert">
          {say("mcp_json_unreadable")}
        </p>
      </Show>
      <ul class="flex flex-col gap-tight">
        <For each={rows()}>
          {(row) => (
            <li class="flex min-w-0 items-center gap-base rounded-card bg-g1 px-base py-snug text-note">
              <span class="w-figure shrink-0 truncate font-mono text-text">{row.draft.label}</span>
              <Badge text={say(SPELLED[row.draft.transport])} />
              <Show
                when={row.encoded.kind === "blocked" ? row.encoded.blocker : undefined}
                fallback={
                  <span class="min-w-0 flex-1 truncate font-mono text-text-faint">
                    {row.draft.url === "" ? row.draft.command : row.draft.url}
                  </span>
                }
              >
                {(blocker) => <span class="min-w-0 flex-1 text-text-faint">{say(WHY[blocker()])}</span>}
              </Show>
            </li>
          )}
        </For>
      </ul>
      <Show
        when={reason()}
        fallback={<Button label={say("mcp_add_all")} tone="primary" onPress={send} />}
      >
        {(why) => (
          <div class="flex min-w-0 items-center gap-base">
            <Button label={say("mcp_add_all")} why={why()} />
            <span class="min-w-0 text-note text-text-faint">{why()}</span>
          </div>
        )}
      </Show>
    </div>
  );
}

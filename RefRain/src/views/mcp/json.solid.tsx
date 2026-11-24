// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Show, createMemo, createSignal } from "solid-js";
import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import Badge from "../parts/badge.solid";
import Button from "../parts/button.solid";
import { SPELLED, WHY, encode, type Intake } from "./draft";
import { readFragment } from "./fragment";

export default function ByJson(props: { readonly intake: Intake }) {
  const lang = storeValue(ui().lang);
  const [text, setText] = createSignal("");
  const fragment = createMemo(() => readFragment(text()));
  const rows = createMemo(() => {
    const value = fragment();
    return value.kind === "unreadable"
      ? []
      : value.drafts.map((draft) => ({ draft, encoded: encode(draft, props.intake.taken()) }));
  });
  const reason = createMemo(
    () =>
      props.intake.why() ??
      (rows().some((row) => row.encoded.kind === "ready")
        ? undefined
        : say(lang(), "mcp_json_empty")),
  );
  function send(): void {
    const offered = rows().flatMap((row) =>
      row.encoded.kind === "ready" ? [row.encoded.server] : [],
    );
    if (offered.length !== 0 && props.intake.offerMany(offered) && offered.length === rows().length)
      setText("");
  }
  return (
    <div class="flex flex-col gap-base">
      <textarea
        class="min-h-output w-full min-w-0 rounded-control border border-edge-input bg-raised px-base py-snug font-mono text-note text-text placeholder:text-text-faint"
        rows={8}
        spellcheck={false}
        aria-label={say(lang(), "mcp_door_json")}
        placeholder={say(lang(), "mcp_json_placeholder")}
        value={text()}
        onInput={(event) => {
          setText(event.currentTarget.value);
        }}
      />
      <Show when={text().trim() !== "" && fragment().kind === "unreadable"}>
        <p class="text-note text-alert" role="alert">
          {say(lang(), "mcp_json_unreadable")}
        </p>
      </Show>
      <ul class="flex flex-col gap-tight">
        <For each={rows()} keyed={(row) => row.draft.label + row.draft.url + row.draft.command}>
          {(row) => {
            const blocker = () => {
              const result = row().encoded;
              return result.kind === "blocked" ? result.blocker : undefined;
            };
            return (
              <li class="flex min-w-0 items-center gap-base rounded-card bg-chrome px-base py-snug text-note">
                <span class="w-figure shrink-0 truncate font-mono text-text">
                  {row().draft.label}
                </span>
                <Badge text={say(lang(), SPELLED[row().draft.transport])} />
                <Show
                  when={blocker()}
                  fallback={
                    <span class="min-w-0 flex-1 truncate font-mono text-text-faint">
                      {row().draft.url || row().draft.command}
                    </span>
                  }
                >
                  {(blocked) => (
                    <span class="min-w-0 flex-1 text-text-faint">
                      {say(lang(), WHY[blocked()])}
                    </span>
                  )}
                </Show>
              </li>
            );
          }}
        </For>
      </ul>
      <Show
        when={reason()}
        fallback={<Button label={say(lang(), "mcp_add_all")} tone="primary" onPress={send} />}
      >
        {(why) => (
          <div class="flex min-w-0 items-center gap-base">
            <Button label={say(lang(), "mcp_add_all")} why={why()} />
            <span class="min-w-0 text-note text-text-faint">{why()}</span>
          </div>
        )}
      </Show>
    </div>
  );
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createSignal, Show } from "solid-js";

import { githubLoginQuery } from "../../core/asking";
import { fill, say, type Key } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { GithubReading, Query } from "../../wire";
import Button from "../parts/button.solid";
import Field from "../parts/field.solid";

export default function ImportLogin(props: {
  readonly onTake: (login: string, host: string) => void;
}) {
  const u = ui();
  const lang = storeValue(u.lang);
  const [host, setHost] = createSignal("");
  const [query, setQuery] = createSignal<Query>();
  const answer = storeValue(() => {
    const question = query();
    return question === undefined ? undefined : u.conn.asking.ask(question);
  });
  const read = () => {
    const held = answer();
    return held !== undefined && "github_login" in held ? held.github_login : undefined;
  };
  const word = (key: Key) => say(lang(), key);

  function ask(): void {
    const asked = githubLoginQuery(host().trim() || null);
    setQuery(asked);
    u.conn.asking.refresh(asked);
  }

  function reason(reading: GithubReading, at: string): string {
    if (reading === "no_cli") return word("you_import_no_cli");
    if (reading === "not_logged_in") return fill(word("you_import_signed_out"), { host: at });
    if (reading === "not_a_host") return fill(word("you_import_not_a_host"), { host: at });
    if ("failed" in reading)
      return fill(word("you_import_failed"), { exit: String(reading.failed.exit ?? "—") });
    if ("stuck" in reading) return fill(word("you_import_stuck"), { why: reading.stuck.why });
    return "";
  }

  return (
    <div class="flex flex-col gap-tight border-t border-edge pt-snug">
      <p class="text-note text-text-faint">{word("you_import_note")}</p>
      <div class="flex flex-wrap items-center gap-snug">
        <div class="min-w-[16ch] flex-1">
          <Field
            label={word("you_import_host")}
            labelling="hidden"
            mono
            placeholder={word("you_import_host_default")}
            value={host()}
            onInput={setHost}
          />
        </div>
        <Button
          label={word("you_import")}
          tone="secondary"
          glyph="search"
          loading={query() !== undefined && answer() === undefined}
          onPress={ask}
        />
      </div>
      <Show when={read()}>
        {(actual) => {
          const found = () => {
            const reading = actual().reading;
            return typeof reading === "object" && "found" in reading ? reading.found : undefined;
          };
          return (
            <div class="flex items-center gap-snug text-note" role="status">
              <Show
                when={found()}
                fallback={
                  <span class="text-text-quiet">{reason(actual().reading, actual().host)}</span>
                }
              >
                {(login) => (
                  <>
                    <span class="min-w-0 flex-1 truncate font-mono text-text">{`${actual().host} · ${login().login}`}</span>
                    <Button
                      label={word("you_import_take")}
                      tone="secondary"
                      glyph="check"
                      onPress={() => props.onTake(login().login, actual().host)}
                    />
                  </>
                )}
              </Show>
            </div>
          );
        }}
      </Show>
    </div>
  );
}

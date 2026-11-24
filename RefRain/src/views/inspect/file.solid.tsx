// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Option, Schema } from "effect";
import { createMemo, Match, Show, Switch } from "solid-js";
import { readDocument } from "../../core/document";
import { fill, say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { kib } from "../../core/time";
import { ui } from "../../ui";
import { Address, type Query } from "../../wire";
import Code from "../parts/code.solid";
import Unanswered from "../parts/unanswered.solid";
import Reach from "./reach.solid";
import type { ReadFrom } from "./reading";

export interface InspectFileProps {
  readonly read: ReadFrom;
}

const WAITING = "…";

export default function InspectFile(props: InspectFileProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const at = createMemo(() =>
    Option.getOrNull(Schema.decodeUnknownOption(Address)(props.read.path)),
  );
  const question = createMemo<Query | null>(() => {
    const address = at();
    return address === null ? null : { document: { at: address } };
  });
  const asked = storeValue(() => {
    const query = question();
    return query === null ? undefined : u.conn.asking.ask(query);
  });
  const read = createMemo(() => readDocument(asked()));
  const unavailable = () => {
    const held = read();
    const query = question();
    return query !== null && held.kind === "unavailable"
      ? { asked: query, query: held.query }
      : undefined;
  };
  const doc = createMemo(() => {
    const held = read();
    if (held.kind !== "held") return null;
    return { ...held.value, lines: held.value.text.split("\n").length };
  });
  const reached = () => props.read.line <= (doc()?.lines ?? 0);

  return (
    <div class="flex min-h-0 flex-1 flex-col bg-page">
      <Switch fallback={<p class="px-wide py-snug text-note text-text-faint">{WAITING}</p>}>
        <Match when={question() === null}>
          <p class="px-wide py-snug text-note text-text-faint">{say(lang(), "file_missing")}</p>
        </Match>
        <Match when={unavailable()}>
          {(result) => (
            <div class="px-wide py-snug">
              <Unanswered query={result().query} asked={result().asked} />
            </div>
          )}
        </Match>
        <Match when={doc()}>
          {(held) => (
            <Show
              when={!held().binary}
              fallback={
                <p class="px-wide py-snug text-note text-text-faint">
                  {fill(say(lang(), "inspect_not_text"), { bytes: kib(held().bytes) })}
                </p>
              }
            >
              <Show when={held().truncated && !reached()}>
                <p class="border-b border-edge px-wide py-tight text-note text-text-faint">
                  {fill(say(lang(), "inspect_line_beyond"), {
                    n: String(props.read.line),
                    bytes: kib(held().bytes),
                  })}
                </p>
              </Show>
              <Code
                path={props.read.path}
                text={held().text}
                cited={reached() ? props.read.line : undefined}
                aside={<Reach path={props.read.path} line={props.read.line} shown="current" />}
              />
            </Show>
          )}
        </Match>
      </Switch>
    </div>
  );
}

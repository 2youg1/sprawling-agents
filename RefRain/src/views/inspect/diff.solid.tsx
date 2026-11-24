// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal, Match, Show, Switch } from "solid-js";
import { readAnswer } from "../../core/answered";
import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Address, Call, Query, RoundsAnswer } from "../../wire";
import { shortOid, type CodeLine } from "../changes";
import { bracketOf } from "../checkpoints";
import Unanswered from "../parts/unanswered.solid";
import Crumb from "./crumb.solid";
import Patch from "./patch.solid";

export interface DiffProps {
  readonly call: Call;
  readonly rounds: RoundsAnswer;
  readonly talk: Address;
}

const WAITING = "…";
const BETWEEN_COMMITS = " ← ";

export default function Diff(props: DiffProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const path = createMemo(() => {
    const render = props.call.render;
    const located =
      render !== null && render !== undefined && typeof render === "object"
        ? render.diff.locations[0]
        : undefined;
    return located ?? props.call.subject ?? "";
  });
  const bracket = createMemo(() => bracketOf(props.rounds, props.call.at));
  const question = createMemo<Query | null>(() => {
    const { base, head } = bracket();
    return base === null || head === null || path() === ""
      ? null
      : { hunks: { oid_a: base, oid_b: head, path: path() } };
  });
  const patchAnswer = storeValue(() => {
    const asked = question();
    return asked === null ? undefined : u.conn.asking.ask(asked);
  });
  const read = createMemo(() =>
    readAnswer(patchAnswer(), (held) => ("hunks" in held ? held.hunks : undefined)),
  );
  const sinceAnswer = storeValue(() => {
    const { head } = bracket();
    return head === null ? undefined : u.conn.asking.ask({ changes: { base: head, head: null } });
  });
  const shown = () => {
    const moved = readAnswer(sinceAnswer(), (held) =>
      "changes" in held ? held.changes.files : undefined,
    );
    return moved.kind === "held" && !moved.value.some((file) => file.path === path())
      ? "current"
      : "past";
  };
  const [cursor, setCursor] = createSignal<CodeLine | null>(null);
  const line = () => {
    const chosen = cursor();
    return chosen === null ? 1 : ((chosen.kind === "removed" ? chosen.old : chosen.new) ?? 1);
  };
  const commitPair = () => {
    const { base, head } = bracket();
    return base === null || head === null ? undefined : { base, head };
  };
  const denied = () => {
    const asked = question();
    const answer = read();
    return asked !== null && answer.kind === "unavailable"
      ? { asked, query: answer.query }
      : undefined;
  };
  const held = () => {
    const answer = read();
    return answer.kind === "held" ? answer.value : undefined;
  };

  return (
    <div class="flex min-h-0 flex-1 flex-col">
      <Crumb path={path()} line={line()} shown={shown()}>
        <Show when={commitPair()}>
          {(pair) => (
            <span class="font-mono">
              {shortOid(pair().head)}
              {BETWEEN_COMMITS}
              {shortOid(pair().base)}
            </span>
          )}
        </Show>
      </Crumb>
      <div class="min-h-0 flex-1 overflow-auto bg-page">
        <Switch fallback={<p class="px-wide py-snug text-note text-text-faint">{WAITING}</p>}>
          <Match when={question() === null}>
            <p class="px-wide py-snug text-note text-text-faint">
              {say(lang(), "run_patch_needs_checkpoint")}
            </p>
          </Match>
          <Match when={denied()}>
            {(lost) => (
              <div class="px-wide py-snug">
                <Unanswered query={lost().query} asked={lost().asked} />
              </div>
            )}
          </Match>
          <Match when={held()}>
            {(patch) => (
              <Patch
                patch={patch()}
                talk={props.talk}
                cursor={cursor()?.number ?? null}
                onCursor={setCursor}
              />
            )}
          </Match>
        </Switch>
      </div>
    </div>
  );
}

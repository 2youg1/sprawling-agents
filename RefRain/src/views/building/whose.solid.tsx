// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, Match, Switch } from "solid-js";
import { readAnswer } from "../../core/answered";
import { fill, say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { GitOid, Query } from "../../wire";
import { shortOid } from "../changes";
import CommitFacts from "./commit_facts.solid";
import CommitLine from "./commit_line.solid";

export default function Whose(props: { readonly oid: GitOid }) {
  const u = ui();
  const lang = storeValue(u.lang);
  const question = createMemo<Query>(() => ({ commit: { oid: props.oid } }));
  const asked = storeValue(() => u.conn.asking.ask(question()));
  const read = createMemo(() =>
    readAnswer(asked(), (answer) => ("commit" in answer ? answer.commit : undefined)),
  );
  const commit = () => {
    const answer = read();
    return answer.kind === "held" ? answer.value : undefined;
  };
  return (
    <Switch fallback={<p class="text-note text-text-faint">…</p>}>
      <Match when={commit()}>
        {(row) => (
          <div class="flex min-w-0 flex-col gap-snug">
            <p class="grid h-control grid-cols-[8ch_minmax(0,1fr)_auto_auto] items-center gap-x-base text-note">
              <CommitLine commit={row()} />
            </p>
            <CommitFacts commit={row()} />
          </div>
        )}
      </Match>
      <Match when={read().kind === "unavailable"}>
        <p class="text-note text-text-faint">
          {fill(say(lang(), "whose_unknown"), { oid: shortOid(props.oid) })}
        </p>
      </Match>
    </Switch>
  );
}

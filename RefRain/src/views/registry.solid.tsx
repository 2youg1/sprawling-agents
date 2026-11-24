// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, Match, Show, Switch } from "solid-js";
import { readAnswer } from "../core/answered";
import { QUERIES } from "../core/asking";
import { fill, say } from "../core/lang";
import { storeValue } from "../core/solid_store";
import { ui } from "../ui";
import Page from "./parts/page.solid";
import type { Rank } from "./parts/page";
import Unanswered from "./parts/unanswered.solid";
import RegistryTable from "./registry/table.solid";

export { RegistryTable };
export interface RegistryProps {
  readonly rank?: Rank | undefined;
}
const WAITING = "…";

export default function Registry(props: RegistryProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const asked = storeValue(u.conn.asking.ask(QUERIES.registry));
  const read = createMemo(() =>
    readAnswer(asked(), (held) => ("registry" in held ? held.registry : undefined)),
  );
  const answer = () => {
    const result = read();
    return result.kind === "held" ? result.value : undefined;
  };
  const denied = () => {
    const result = read();
    return result.kind === "unavailable" ? result : undefined;
  };
  const count = () => answer()?.assets.length ?? 0;
  return (
    <Page
      title={say(lang(), "nav_registry")}
      note={say(lang(), "registry_note")}
      rank={props.rank}
      aside={
        <Show when={count() > 0}>
          <span class="figure text-note text-text-quiet">
            {fill(say(lang(), "registry_count"), { n: String(count()) })}
          </span>
        </Show>
      }
    >
      <Switch fallback={<p class="text-text-faint">{WAITING}</p>}>
        <Match when={denied()}>
          {(lost) => <Unanswered query={lost().query} asked={QUERIES.registry} />}
        </Match>
        <Match when={answer()}>{(held) => <RegistryTable assets={held().assets} />}</Match>
      </Switch>
    </Page>
  );
}

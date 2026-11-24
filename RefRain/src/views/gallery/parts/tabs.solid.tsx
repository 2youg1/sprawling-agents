// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal, Match, Switch } from "solid-js";
import { say, type Key } from "../../../core/lang";
import type { RunLens } from "../../../core/route";
import { storeValue } from "../../../core/solid_store";
import { ui } from "../../../ui";
import Badge from "../../parts/badge.solid";
import Button from "../../parts/button.solid";
import type { Lens } from "../../parts/tab_navigation";
import Tabs from "../../parts/tabs.solid";
import { GalleryCase } from "../case";

const WORDS: readonly (readonly [RunLens, Key])[] = [
  ["time", "run_time"],
  ["turns", "run_turns"],
  ["monitor", "run_monitor"],
  ["prompt", "run_prompt"],
  ["context", "run_context"],
  ["changes", "run_changes"],
  ["evidence", "run_evidence"],
];

export default function TabsCases() {
  const lang = storeValue(ui().lang);
  const all = createMemo(() => WORDS.map(([id, key]): Lens => ({ id, label: say(lang(), key) })));
  const three = createMemo(() =>
    all().filter((lens) => lens.id === "time" || lens.id === "turns" || lens.id === "changes"),
  );
  const [held, setHeld] = createSignal("turns");
  const [plain, setPlain] = createSignal("time");
  const [narrow, setNarrow] = createSignal("evidence");
  const label = () => say(lang(), "run_lenses");
  const panel = (lens: Lens) => (
    <Switch
      fallback={<p class="pt-snug text-note text-text-quiet">{say(lang(), "commits_empty")}</p>}
    >
      <Match when={lens.id === "turns"}>
        <div class="flex items-center gap-snug pt-snug">
          <Button label={say(lang(), "run_evidence")} tone="secondary" onPress={() => undefined} />
        </div>
      </Match>
      <Match when={lens.id === "time"}>
        <p class="pt-snug text-note text-text-quiet">{say(lang(), "run_no_turns")}</p>
      </Match>
    </Switch>
  );
  return (
    <>
      <GalleryCase label="tabs · three lenses, a control in the panel and a count beside a name">
        <Tabs
          label={label()}
          lenses={three()}
          current={held()}
          onPick={setHeld}
          panel={panel}
          mark={(lens) => (lens.id === "turns" ? <Badge text="12" /> : undefined)}
        />
      </GalleryCase>
      <GalleryCase label="tabs · a panel of plain text is a stop of its own">
        <Tabs label={label()} lenses={three()} current={plain()} onPick={setPlain} panel={panel} />
      </GalleryCase>
      <GalleryCase label="tabs · seven lenses at a phone's width" width={390}>
        <Tabs label={label()} lenses={all()} current={narrow()} onPick={setNarrow} panel={panel} />
      </GalleryCase>
    </>
  );
}

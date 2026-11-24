// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Show, createMemo, createSignal } from "solid-js";
import { say, type Key } from "../core/lang";
import type { Lens } from "../core/route";
import { storeValue } from "../core/solid_store";
import { ui } from "../ui";
import Page from "./parts/page.solid";
import Tabs from "./parts/tabs.solid";
import { Archive, Bin } from "./record/history_views";
import type { Source } from "./record/timeline";
import Timeline from "./record/timeline.solid";

const READINGS = ["timeline", "archive", "bin"] as const;
const NAMES: Record<(typeof READINGS)[number], Key> = {
  timeline: "rec_timeline",
  archive: "rec_archive",
  bin: "rec_bin",
};

function reading(lens: Lens): (typeof READINGS)[number] {
  return lens === "archive" || lens === "bin" ? lens : "timeline";
}

function route(source: Source): Lens {
  return source === "log" ? "log" : "ledger";
}

export default function RecordView(props: {
  readonly lens: Lens;
  readonly rank?: "page" | "section" | undefined;
}) {
  const u = ui();
  const lang = storeValue(u.lang);
  const [picked, setPicked] = createSignal<Source>("every");
  const source = createMemo<Source>(() =>
    props.lens === "log" ? "log" : picked() === "log" ? "every" : picked(),
  );
  const tabs = createMemo(() => READINGS.map((id) => ({ id, label: say(lang(), NAMES[id]) })));
  function pick(id: string): void {
    const found = READINGS.find((each) => each === id);
    if (found === undefined || found === reading(props.lens)) return;
    u.go({ kind: "record", lens: found === "timeline" ? route(source()) : found });
  }
  function choose(value: Source): void {
    setPicked(value);
    if (route(value) !== props.lens) u.go({ kind: "record", lens: route(value) });
  }
  return (
    <Page title={say(lang(), "nav_the_record")} rank={props.rank}>
      <Tabs
        label={say(lang(), "rec_lenses")}
        lenses={tabs()}
        current={reading(props.lens)}
        onPick={pick}
        panel={(tab) => (
          <>
            <Show when={tab.id === "timeline"}>
              <Timeline source={source()} onSource={choose} />
            </Show>
            <Show when={tab.id === "archive"}>
              <Archive />
            </Show>
            <Show when={tab.id === "bin"}>
              <Bin />
            </Show>
          </>
        )}
      />
    </Page>
  );
}

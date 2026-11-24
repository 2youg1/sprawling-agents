// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, lazy, Loading, Match, Show, Switch } from "solid-js";
import { say } from "../core/lang";
import { storeValue } from "../core/solid_store";
import type { View } from "../core/route";
import { ui } from "../ui";
import { backLookOf } from "./pages/back";
import Back from "./pages/back.look.solid";
import Building from "./building.solid";
import Raise from "./raise.solid";
import City from "./city.solid";
import Cost from "./cost.solid";
import Mcp from "./mcp.solid";
import Monitor from "./monitor.solid";
import RecordView from "./record.solid";
import Registry from "./registry.solid";
import Run from "./run.solid";
import Welcome from "./welcome.solid";

const Gallery = lazy(() => import("./gallery.solid"));

function variant<K extends View["kind"]>(page: View, kind: K): page is Extract<View, { kind: K }> {
  return page.kind === kind;
}

export default function Pages(props: { readonly view: View }) {
  const u = ui();
  const lang = storeValue(u.lang);
  const samples = storeValue(u.conn.monitor.samples);
  function page<K extends View["kind"]>(kind: K) {
    const now = props.view;
    return variant(now, kind) ? now : undefined;
  }
  const back = createMemo(() => backLookOf(props.view, lang()));
  return (
    <main
      id="main"
      class="col-[2/12] row-start-2 flex min-h-0 min-w-0 flex-col overflow-y-auto narrow:col-span-full"
      aria-label={say(lang(), "region_main")}
    >
      <Show when={back()}>
        {(look) => (
          <div class="mb-snug">
            <Back {...look()} />
          </div>
        )}
      </Show>
      <Switch>
        <Match when={page("city")}>
          <City />
        </Match>
        <Match when={page("building")}>
          {(now) => <Building address={now().address} initialSection={now().section} />}
        </Match>
        <Match when={page("raise")}>{(now) => <Raise address={now().address} />}</Match>
        <Match when={page("run")}>
          {(now) => (
            <Show when={now().run} keyed>
              {(run) => <Run run={run} lens={now().lens} />}
            </Show>
          )}
        </Match>
        <Match when={page("mcp")}>
          <Mcp />
        </Match>
        <Match when={page("record")}>{(now) => <RecordView lens={now().lens} />}</Match>
        <Match when={page("cost")}>
          <Cost />
        </Match>
        <Match when={page("registry")}>
          <Registry />
        </Match>
        <Match when={page("welcome")}>{(now) => <Welcome step={now().step} />}</Match>
        <Match when={page("monitor")}>
          <Monitor samples={samples()} watch={u.conn.monitor.watch} beat={u.conn.monitor.setBeat} />
        </Match>
        <Match when={page("gallery")}>
          <Loading fallback={<div data-pending="" />}>
            <Gallery />
          </Loading>
        </Match>
      </Switch>
    </main>
  );
}

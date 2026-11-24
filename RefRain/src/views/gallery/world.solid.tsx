// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createSignal, onSettled } from "solid-js";
import { say } from "../../core/lang";
import { roomOf } from "../../core/route";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import Tabs from "../parts/tabs.solid";
import Divider from "../world/divider.look.solid";
import { dividerLookOf, lineAt } from "../world/divider";
import PaneMenu from "../world/pane_menu.solid";
import { ROOM } from "./benched";
import { GalleryCase } from "./case";
import { pressing } from "./pressing";

export default function WorldPaneCases() {
  const lang = storeValue(ui().lang);
  const [span, setSpan] = createSignal(3);
  const [dragging, setDragging] = createSignal(false);
  const [selected, setSelected] = createSignal("commits");
  let menu: HTMLDivElement | undefined;
  let grid: HTMLDivElement | undefined;

  onSettled(() =>
    menu === undefined ? undefined : pressing('button[aria-haspopup="menu"]')(menu),
  );
  const tracks = () => String(span() + 1);
  const look = () =>
    dividerLookOf(
      {
        label: say(lang(), "world_sessions"),
        controls: "gallery-sessions",
        span: span(),
        widest: 8,
        dragging: dragging(),
      },
      {
        set: setSpan,
        reset: () => {
          setSpan(3);
        },
        drag: setDragging,
        follow: (client) => {
          if (grid !== undefined) setSpan(Math.min(8, Math.max(3, lineAt(grid, client) - 1)));
        },
      },
    );
  return (
    <>
      <GalleryCase label="workbench · a pane's label, its menu open on the middle pane" width={640}>
        <div
          class="flex h-[160px] flex-col px-base"
          ref={(node) => {
            menu = node;
          }}
        >
          <PaneMenu pane="session" label={say(lang(), "world_session")} arranged="menu" />
        </div>
      </GalleryCase>
      <GalleryCase label="workbench · a pane's label in the blend tier, words only" width={640}>
        <div class="flex flex-col px-base">
          <PaneMenu pane="sessions" label={say(lang(), "world_sessions")} arranged="words" />
        </div>
      </GalleryCase>
      <GalleryCase label="workbench · the divider between the first two panes" width={1440}>
        <div
          class="frame grid h-[160px] grid-rows-[minmax(0,1fr)_auto]"
          ref={(node) => {
            grid = node;
          }}
        >
          <div
            id="gallery-sessions"
            class="row-[1/3] rounded-panel border border-edge-panel"
            style={{ "grid-column": `1 / ${tracks()}` }}
          />
          <div
            class="z-(--layer-raised) row-[1/3] -ml-gutter flex w-gutter"
            style={{ "grid-column": tracks() }}
          >
            <Divider {...look()} />
          </div>
          <div
            class="row-[1/3] rounded-panel border border-edge-panel"
            style={{ "grid-column": `${tracks()} / 10` }}
          />
        </div>
      </GalleryCase>
      <GalleryCase label="workbench · the world sheet's head on a phone" width={390}>
        <Tabs
          label={say(lang(), "world_place")}
          lenses={[
            { id: "sessions", label: say(lang(), "world_sessions") },
            { id: "session", label: say(lang(), "world_session") },
            { id: "commits", label: roomOf(ROOM) },
          ]}
          current={selected()}
          onPick={setSelected}
          panel={() => null}
        />
      </GalleryCase>
    </>
  );
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createUniqueId, onSettled, Show } from "solid-js";

import { say } from "../core/lang";
import { ui } from "../ui";
import type { Seat } from "./modal";
import { boxOf, workspaceProblem, type BoxWire } from "./palette";
import { paletteController, type PaletteInput } from "./palette/controller";
import CommandDialog from "./palette/dialog";
import { nothingFound } from "./palette/nothing_found";
import { rowId, rowsLookOf } from "./palette/rows";
import { PaletteRows } from "./palette/rows_view";
import type { Arrival } from "./palette_door";
import Empty from "./parts/empty.solid";
import Listening from "./talk/listening_view";

export interface PaletteProps extends PaletteInput {
  readonly seat?: Seat | undefined;
  readonly arrival?: Arrival | undefined;
}

export default function Palette(props: PaletteProps) {
  const u = ui();
  const state = paletteController(props);
  const list = createUniqueId();
  const seat = () => props.seat ?? "modal";
  const verbs = createMemo(() => state.query().trim().startsWith("/") && !state.carrying());
  const empty = createMemo(() => {
    if (state.shown().length > 0) return undefined;
    const query = state.query();
    return state.carrying() || verbs()
      ? nothingFound(query)
      : (workspaceProblem(query, state.buildings() ?? []) ?? nothingFound(query));
  });
  const foot = createMemo(() =>
    state.query().trim() === "" && !state.carrying() ? state.here() : null,
  );
  const box = createMemo(() =>
    boxOf(
      {
        asking: state.carrying() ? "carry" : "places",
        query: state.query(),
        cursor: state.cursor(),
        count: state.shown().length,
        list,
        lang: state.lang(),
      },
      {
        write: state.write,
        point: (at) => {
          state.point(at);
          requestAnimationFrame(() =>
            document.getElementById(rowId(list, at))?.scrollIntoView({ block: "nearest" }),
          );
        },
        pick: () => state.pick(state.shown().at(state.cursor())),
      },
    ),
  );
  const look = createMemo(() =>
    rowsLookOf(
      {
        listing: verbs() ? { kind: "verbs", groups: state.grouped() } : { kind: "places" },
        shown: state.shown(),
        cursor: state.cursor(),
        id: list,
        lang: state.lang(),
      },
      { hover: state.point, pick: state.pick },
    ),
  );
  onSettled(() => {
    if (seat() === "modal") u.prefs.findPalette();
  });
  return (
    <CommandDialog
      seat={seat()}
      named={{ "aria-label": say(state.lang(), "nav_palette") }}
      backdrop="closes"
      first={props.arrival === "touch" ? "sheet" : "control"}
      onClose={props.onClose}
    >
      <PaletteBox box={box()} />
      <PaletteRows look={look()} />
      <Show when={empty()}>{(missing) => <Empty missing={missing()} />}</Show>
      <Show when={foot()}>
        {(room) => (
          <div class="mt-snug border-t border-edge-panel pt-snug">
            <Listening room={room()} />
          </div>
        )}
      </Show>
    </CommandDialog>
  );
}

export function PaletteBox(props: { readonly box: BoxWire }) {
  return (
    <input
      role={props.box.role}
      aria-autocomplete={props.box["aria-autocomplete"]}
      aria-expanded={props.box["aria-expanded"] ? "true" : "false"}
      aria-controls={props.box["aria-controls"]}
      aria-activedescendant={props.box["aria-activedescendant"]}
      aria-label={props.box["aria-label"]}
      placeholder={props.box.placeholder}
      value={props.box.value}
      onInput={(event) => props.box.oninput(event)}
      onKeyDown={(event) => props.box.onkeydown(event)}
      class="w-full rounded-control bg-page px-base py-snug text-body placeholder:text-text-faint"
    />
  );
}

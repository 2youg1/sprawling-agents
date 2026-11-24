// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import {
  createEffect,
  createMemo,
  createSignal,
  createUniqueId,
  onSettled,
  untrack,
} from "solid-js";

import {
  lookOf,
  matching,
  rowId,
  type Answer,
  type Choice,
  type ComboboxProps,
  type Hands,
  type Layer,
  type Return,
} from "./combobox";
import ComboboxControls, { type BoxRefs } from "./combobox_controls";
import { revealIn, sideFor, type Side } from "./layer";

const PREFERRED: Side = "below";

export default function Combobox(props: ComboboxProps) {
  const uid = createUniqueId();
  const started = untrack(() => props.starts) === "open";
  const [layer, setLayer] = createSignal<Layer>(started ? "open" : "unopened");
  const [query, setQuery] = createSignal("");
  const [at, setAt] = createSignal(0);
  const [kept, setKept] = createSignal<readonly Choice[]>([]);
  const [side, setSide] = createSignal<Side>(PREFERRED);
  let root: HTMLDivElement | undefined;
  let trigger: HTMLButtonElement | undefined;
  let panel: HTMLDivElement | undefined;
  let search: HTMLInputElement | undefined;
  let list: HTMLUListElement | undefined;
  let focusOnOpen = false;

  const live = createMemo(() => matching(props.choices, query()));
  const rows = createMemo(() => (layer() === "open" ? live() : kept()));
  const cursor = createMemo(() => Math.max(0, Math.min(at(), rows().length - 1)));

  function close(focus: Return): void {
    setKept(live());
    setLayer("shut");
    setQuery("");
    setAt(0);
    if (focus === "opener") trigger?.focus();
  }

  function take(choice: Choice | undefined): void {
    if (choice === undefined) return;
    props.onPick(choice.value);
    close("opener");
  }

  function act(answered: Answer): void {
    switch (answered.kind) {
      case "cursor":
        setAt(answered.at);
        return;
      case "take":
        take(rows().at(cursor()));
        return;
      case "close":
        close(answered.focus);
        return;
      case "pass":
        return;
    }
  }

  createEffect(
    () => layer(),
    (state) => {
      if (state !== "open") return;
      onSettled(() => {
        if (panel === undefined || layer() !== "open") return;
        setSide(sideFor(panel, PREFERRED));
        if (focusOnOpen) {
          focusOnOpen = false;
          search?.focus({ preventScroll: true });
        }
      });
    },
  );

  createEffect(
    () => ({ open: layer() === "open", cursor: cursor(), count: rows().length }),
    ({ open, cursor, count }) => {
      if (!open || count === 0) return;
      onSettled(() => {
        if (list === undefined || layer() !== "open") return;
        const row = list.ownerDocument.getElementById(rowId(uid, cursor));
        if (row !== null) revealIn(list, row);
      });
    },
  );

  const hands: Hands = {
    toggle: () => {
      if (layer() === "open") {
        close("opener");
        return;
      }
      focusOnOpen = true;
      setSide(PREFERRED);
      setLayer("open");
    },
    act,
    type: (typed) => {
      setQuery(typed);
      setAt(0);
    },
    point: setAt,
    take,
    leave: (next) => {
      if (layer() !== "open") return;
      if (next instanceof Node && root?.contains(next) === true) return;
      close("leave");
    },
  };

  const holds: BoxRefs = {
    root: (node) => {
      root = node;
    },
    trigger: (node) => {
      trigger = node;
    },
    layer: (node) => {
      panel = node;
    },
    search: (node) => {
      search = node;
    },
    list: (node) => {
      list = node;
    },
  };

  const look = createMemo(() =>
    lookOf(
      {
        props,
        uid,
        layer: layer(),
        query: query(),
        cursor: cursor(),
        rows: rows(),
        side: side(),
      },
      hands,
    ),
  );
  return <ComboboxControls {...look()} holds={holds} />;
}

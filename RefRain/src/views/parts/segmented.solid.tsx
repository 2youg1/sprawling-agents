// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createUniqueId, onCleanup } from "solid-js";
import { lookOf } from "./segmented";
import type { HoldCell, SegmentedProps } from "./segmented";
import Look from "./segmented.look.solid";

export default function Segmented<V extends string>(props: SegmentedProps<V>) {
  const uid = createUniqueId();
  const drawn = new Map<V, HTMLButtonElement>();
  const refs = new Map<V, HoldCell>();
  function hold(value: V): HoldCell {
    const kept = refs.get(value);
    if (kept !== undefined) return kept;
    const wired: HoldCell = (node) => {
      drawn.set(value, node);
      onCleanup(() => {
        if (drawn.get(value) === node) drawn.delete(value);
      });
    };
    refs.set(value, wired);
    return wired;
  }
  const look = createMemo(() =>
    lookOf(props, uid, {
      focus: (value) => drawn.get(value)?.focus(),
      hold,
    }),
  );
  return <Look {...look()} />;
}

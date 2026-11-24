// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo } from "solid-js";
import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import { lookOf, type Grip, type SplitProps } from "./split";
import Look from "./split.look.solid";

export default function InspectorSplit(props: SplitProps) {
  const lang = storeValue(ui().lang);
  let line: HTMLElement | undefined;
  const grip: { from: Grip | null } = { from: null };
  const look = createMemo(() =>
    lookOf(props, say(lang(), "inspect_split"), {
      capture: (pointer) => line?.setPointerCapture(pointer),
      hold: (node) => {
        line = node;
        return () => {
          if (line === node) line = undefined;
        };
      },
      grip,
    }),
  );
  return <Look {...look()} />;
}

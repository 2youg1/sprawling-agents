// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal, onCleanup } from "solid-js";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import Look from "./copy.look.solid";
import { lookOf, presser, REST, textOf, type Copied, type CopyProps } from "./copy";

export default function Copy(props: CopyProps) {
  const lang = storeValue(ui().lang);
  const [copied, setCopied] = createSignal<Copied>(REST);
  let waiting: ReturnType<typeof setTimeout> | undefined;
  let active = true;
  onCleanup(() => {
    active = false;
    clearTimeout(waiting);
  });
  const press = presser({
    board: () => navigator.clipboard,
    later: (run, ms) => {
      if (!active) return () => undefined;
      const receipt = setTimeout(run, ms);
      waiting = receipt;
      return () => clearTimeout(receipt);
    },
    show: (now) => {
      if (active) setCopied(now);
    },
  });
  const look = createMemo(() => lookOf(props, copied(), lang(), () => press(textOf(props.text))));
  return <Look {...look()} />;
}

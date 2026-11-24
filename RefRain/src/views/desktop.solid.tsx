// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal } from "solid-js";
import { configureDesktop } from "../core/commands";
import { readDocument } from "../core/document";
import { storeValue } from "../core/solid_store";
import { ui } from "../ui";
import type { Address } from "../wire";
import { desktopScopeAt, lookOf } from "./desktop";
import Look from "./desktop.look.solid";

export default function DesktopForm(props: { readonly addr: Address }) {
  const u = ui();
  const lang = storeValue(u.lang);
  const [draft, setDraft] = createSignal("");
  const [edited, setEdited] = createSignal(false);
  const answer = storeValue(() =>
    u.conn.asking.ask({ document: { at: desktopScopeAt(props.addr) } }),
  );
  const read = createMemo(() => readDocument(answer()));
  const onDisk = createMemo(() => {
    const current = read();
    return current.kind === "held" ? current.value.text : "";
  });
  let previous: Address | undefined;
  createEffect(
    () => [props.addr, onDisk()] as const,
    ([addr, text]) => {
      if (!edited() || previous !== addr) {
        setEdited(false);
        setDraft(text);
      }
      previous = addr;
    },
  );
  function save(): void {
    if (u.send(configureDesktop(props.addr, draft()))) setEdited(false);
  }
  const file = createMemo(() => {
    switch (read().kind) {
      case "unavailable":
        return "missing";
      case "held":
        return onDisk() === "" ? "empty" : "held";
      case "asking":
        return "held";
    }
  });
  const look = createMemo(() =>
    lookOf({ addr: props.addr, draft: draft(), edited: edited(), file: file() }, lang(), {
      type: (text) => {
        setDraft(text);
        setEdited(true);
      },
      save,
    }),
  );
  return <Look {...look()} />;
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal, onCleanup } from "solid-js";
import { reachOf, type Shown } from "../../core/editor";
import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { TextKeyLook } from "./text_key";
import TextKey from "./text_key.look.solid";

export interface ReachProps {
  readonly path: string;
  readonly line: number;
  readonly shown: Shown;
}

const RECEIPT_MS = 1200;

export default function Reach(props: ReachProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const editor = storeValue(u.prefs.editorHeld);
  const [copied, setCopied] = createSignal(false);
  let receipt: ReturnType<typeof setTimeout> | undefined;
  let active = true;
  onCleanup(() => {
    active = false;
    clearTimeout(receipt);
  });

  function copy(text: string): void {
    void navigator.clipboard.writeText(text).then(
      () => {
        if (!active) return;
        setCopied(true);
        clearTimeout(receipt);
        receipt = setTimeout(() => setCopied(false), RECEIPT_MS);
      },
      () => setCopied(false),
    );
  }

  const look = createMemo<TextKeyLook>(() => {
    const place = reachOf({ ...editor(), path: props.path, line: props.line }, props.shown);
    if (place.kind === "link")
      return { label: say(lang(), "setup_editor"), wire: { href: place.href } };
    return {
      label: say(lang(), copied() ? "code_copied" : "inspect_copy_place"),
      wire: { type: "button", onclick: () => copy(place.text) },
    };
  });
  return <TextKey {...look()} />;
}

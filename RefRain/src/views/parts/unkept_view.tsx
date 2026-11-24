// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import Copy from "./copy.solid";
import type { UnkeptLook } from "./unkept";

export function UnkeptLookView(props: UnkeptLook) {
  return (
    <p class="flex items-center gap-base text-note text-alert" role={props.line.role}>
      <span class="min-w-0 flex-1">{props.sentence}</span>
      <Copy {...props.copy} />
    </p>
  );
}

export default function Unkept(props: { readonly words: () => string }) {
  const lang = storeValue(ui().lang);
  return (
    <UnkeptLookView
      sentence={say(lang(), "part_unkept")}
      line={{ role: "status" }}
      copy={{ text: props.words, note: say(lang(), "unkept_copy_note") }}
    />
  );
}

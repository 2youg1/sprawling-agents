// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import type { ConflictWire } from "./conflict";

interface ConflictLookProps {
  readonly text: string;
  readonly wire: ConflictWire;
  readonly actions: JSX.Element;
}

export default function ConflictLook(props: ConflictLookProps) {
  return (
    <div {...props.wire} class="flex items-center gap-snug text-note text-text asks">
      <p class="min-w-0 flex-1">{props.text}</p>
      {props.actions}
    </div>
  );
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";

export interface LineProps {
  readonly tone: "faint" | "alert";
  readonly children: JSX.Element;
}

export default function Line(props: LineProps) {
  return (
    <div
      class="line border-b border-edge px-wide py-tight text-note text-text-faint data-[tone=alert]:text-alert"
      data-tone={props.tone}
    >
      {props.children}
    </div>
  );
}

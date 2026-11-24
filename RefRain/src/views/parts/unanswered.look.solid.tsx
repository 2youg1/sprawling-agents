// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Show } from "solid-js";

import Button from "./button.solid";
import type { UnansweredLook } from "./unanswered";

export default function Look(props: UnansweredLook) {
  return (
    <div class="flex flex-col items-start gap-snug text-note">
      <p class="text-text-faint">{props.said}</p>
      <Show when={props.reason !== undefined}>
        <p class="font-mono text-text-faint wrap-anywhere">{props.reason}</p>
      </Show>
      <Button
        label={props.retry.label}
        tone="quiet"
        onPress={() => {
          props.retry.press();
        }}
      />
    </div>
  );
}

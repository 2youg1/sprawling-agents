// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createUniqueId, Show } from "solid-js";

import ControlTip from "../parts/control_tip";
import { KbdMarks } from "../parts/kbd.look";
import { chordMarks } from "../parts/kbd_marks";
import type { ChordLook } from "./chord";

export default function ChordView(props: ChordLook) {
  const tip = createUniqueId();
  return (
    <ControlTip id={tip} text={props.tip}>
      <button
        data-control=""
        type={props.wire.type}
        aria-pressed={props.wire["aria-pressed"] ? "true" : "false"}
        aria-describedby={tip}
        onClick={() => props.wire.onclick()}
        onBlur={() => props.wire.onblur()}
        onKeyDown={(event) => {
          if (!event.isComposing) props.wire.onkeydown(event);
        }}
        class="h-control-sm rounded-control border border-edge px-snug transition-colors duration-short ease-leave hover:bg-raised hover:ease-arrive aria-pressed:bg-raised-hover"
      >
        <Show
          when={props.word === undefined && props.spelled}
          keyed
          fallback={<span class="font-mono text-note text-text-faint">{props.word}</span>}
        >
          {(_spelled) => <KbdMarks marks={chordMarks({ action: props.action })} />}
        </Show>
      </button>
    </ControlTip>
  );
}

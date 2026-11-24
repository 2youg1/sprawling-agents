// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { keymap, marks } from "../../core/keys";
import type { Action } from "../../core/keys";
import { lineFaces } from "../../core/lines";
import type { LineMove } from "../../core/lines";

export type KbdProps = (
  { readonly action: Action } | { readonly move: LineMove } | { readonly initial: string }
) & { readonly class?: string | undefined };

export function chordMarks(props: KbdProps): readonly string[] {
  const keys = keymap();
  if ("move" in props) return lineFaces(props.move).slice(0, 1);
  if ("initial" in props) {
    return marks({ accel: false, shift: false, key: props.initial }, keys.platform);
  }
  return marks(keys.chord(props.action), keys.platform);
}

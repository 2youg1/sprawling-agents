// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Lang } from "../../core/lang";
import { fill, say } from "../../core/lang";
import { count } from "../../core/time";

export interface ReasoningLook {
  readonly label: string;
  readonly length: string;
  readonly text: string;
  readonly open: boolean;
}

const utf8 = new TextEncoder();

export function lengthOf(lang: Lang, text: string): string {
  return fill(say(lang, "talk_reasoning_length"), { n: count(utf8.encode(text).length) });
}

export function lookOf(lang: Lang, text: string, open: boolean): ReasoningLook {
  return { label: say(lang, "talk_reasoning"), length: lengthOf(lang, text), text, open };
}

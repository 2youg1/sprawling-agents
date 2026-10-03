// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// How the queued-signal section says a signal's kind and its count
// (client D86). `SignalLine.kind` is the wire word `kernel::SignalKind`
// spells; a word this build has no phrase for is shown as written, so a
// new kind reads as itself rather than as nothing.

import { fill, isKey, say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import { count } from "../../core/time";

export function kindSaid(lang: Lang, kind: string): string {
  const key = `signal_kind_${kind}`;
  return isKey(key) ? say(lang, key) : kind;
}

export function waitingSaid(lang: Lang, n: number): string {
  return n === 1 ? say(lang, "inbox_count_one") : fill(say(lang, "inbox_count"), { n: count(n) });
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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

export interface InboxRow {
  readonly key: string;
  readonly href: string | undefined;
  readonly kind: string;
  readonly first: string;
  readonly from: string;
  readonly ago: string;
}

export interface InboxLook {
  readonly label: string;
  readonly heading: string;
  readonly rows: readonly InboxRow[];
}

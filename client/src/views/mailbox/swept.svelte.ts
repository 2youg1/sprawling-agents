// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the person put out of the mailbox, by notice key and the count
// it had then. A refusal that arrives again is news again, so a repeat
// past the swept count shows; nothing here deletes the city's answer,
// which stays in the belief. Module state, so the deciding section and
// the notices section, which each sweep their own, read one record.

import type { Notice } from "../../core/belief";

let swept = $state.raw<Readonly<Record<string, number>>>({});

export function shown(notice: Notice): boolean {
  return notice.count > (swept[notice.key] ?? 0);
}

export function sweep(notices: readonly Notice[]): void {
  const next: Record<string, number> = { ...swept };
  for (const notice of notices) next[notice.key] = notice.count;
  swept = next;
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What an approval card's question is about, as its look is given it
// (`asked.look.svelte`): still being asked for, a binary the card can
// only weigh, or the text, with what the cut left out when the content
// answer was cut.

import { fill, say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import { kib } from "../../core/time";
import type { ContentAnswer } from "../../wire";

export type Shown =
  | { readonly kind: "asking" }
  | { readonly kind: "binary"; readonly said: string }
  | { readonly kind: "text"; readonly text: string; readonly cut: string | undefined };

export interface AskedLook {
  readonly summary: string;
  readonly shown: Shown;
}

export function shownOf(lang: Lang, content: ContentAnswer | undefined): Shown {
  if (content === undefined) return { kind: "asking" };
  if (content.binary) return { kind: "binary", said: fill(say(lang, "file_binary"), { kib: kib(content.bytes) }) };
  return {
    kind: "text",
    text: content.text,
    cut: content.truncated
      ? fill(say(lang, "file_truncated"), { kib: kib(content.text.length), total: kib(content.bytes) })
      : undefined,
  };
}

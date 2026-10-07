// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The picker above a PDF or a DOCX that names the other version it is
// compared with (client/Spec.lean §4-61): what `against.look.svelte`
// draws, every word already in the person's language and the list's
// value and hand in one wire bag the look spreads on it.

import { say } from "../../../core/lang";
import type { Lang } from "../../../core/lang";
import type { B3Hash, DocumentVersion } from "../../../wire";
import { short } from "../reading";

// The bag spread on the list: the version it holds, or the empty value
// for no comparison, and the hand that picks another.
export interface AgainstWire {
  readonly value: string;
  readonly onchange: (event: { readonly currentTarget: { readonly value: string } }) => void;
}

export interface AgainstLook {
  readonly label: string;
  // The entry that compares with nothing, first in the list.
  readonly none: string;
  readonly options: readonly { readonly value: B3Hash; readonly label: string }[];
  readonly wire: AgainstWire;
}

export function againstLookOf(
  others: readonly DocumentVersion[],
  against: B3Hash | null,
  lang: Lang,
  pick: (version: B3Hash | null) => void,
): AgainstLook {
  return {
    label: say(lang, "format_compare_with"),
    none: say(lang, "format_compare_none"),
    options: others.map((each) => ({ value: each.version, label: short(each.version) })),
    wire: {
      value: against ?? "",
      onchange: (event) => {
        pick(others.find((each) => each.version === event.currentTarget.value)?.version ?? null);
      },
    },
  };
}

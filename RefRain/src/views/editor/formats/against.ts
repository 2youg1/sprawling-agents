// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import { say } from "../../../core/lang";
import type { Lang } from "../../../core/lang";
import type { B3Hash, DocumentVersion } from "../../../wire";
import { short } from "../reading";


export interface AgainstWire {
  readonly value: string;
  readonly onchange: (event: { readonly currentTarget: { readonly value: string } }) => void;
}

export interface AgainstLook {
  readonly label: string;
  
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

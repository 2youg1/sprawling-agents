// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export interface SectionWire {
  readonly type: "button";
  readonly "aria-current": "true" | undefined;
  readonly onclick: () => void;
}

export interface SectionLook {
  readonly key: string;
  readonly label: string;
  readonly current: boolean;
  readonly wire: SectionWire;
}

export interface SectionsLook {
  readonly entries: readonly SectionLook[];
}

export function sectionsLookOf<S extends string>(
  sections: readonly { readonly key: S; readonly label: string }[],
  current: S | null,
  pick: (section: S) => void,
): SectionsLook {
  return {
    entries: sections.map((section) => ({
      key: section.key,
      label: section.label,
      current: section.key === current,
      wire: {
        type: "button",
        "aria-current": section.key === current ? "true" : undefined,
        onclick: () => {
          pick(section.key);
        },
      },
    })),
  };
}

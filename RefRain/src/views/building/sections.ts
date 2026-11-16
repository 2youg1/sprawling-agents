// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the entries of the building page's index are given (client D95:
// the page is the seat and keeps the landmark, `sections.look.svelte`
// draws the entries): one entry per section, the one in
// the middle column marked current. The middle column may show what the
// tree picked instead, and then no entry is current.

export interface SectionWire {
  readonly type: "button";
  readonly "aria-current": "true" | undefined;
  readonly onclick: () => void;
}

export interface SectionLook {
  readonly key: string;
  // Already in the person's language.
  readonly label: string;
  readonly current: boolean;
  readonly wire: SectionWire;
}

export interface SectionsLook {
  readonly entries: readonly SectionLook[];
}

// The index over `sections`, with `current` the one in the middle column
// (or `null` when the tree's pick is there) and `pick` what a press asks.
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

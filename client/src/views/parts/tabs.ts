// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a set of tabs decides before anything is drawn, and the whole
// value a look receives (`TabsLook`): where a key lands, which tab is
// the one Tab stop, which panel is a stop of its own, and the wire bag
// each element carries. A look spreads each bag on the element it names
// and adds nothing to it, so another look - one built from a component
// library - draws the same tabs with the same keys and the same
// associations (client/spec/Views/Parts.lean §7-4, the keys modelled in
// client/spec/Views/Parts/Tabs.lean).

import type { Snippet } from "svelte";
import { createAttachmentKey } from "svelte/attachments";
import type { Attachment } from "svelte/attachments";

// One lens of the subject. Content does not travel in this row: it
// arrives as the `panel` and `mark` snippets, because a snippet is
// written in markup while a row like this is built in script.
export interface Lens {
  readonly id: string;
  // Already in the person's language.
  readonly label: string;
}

export interface TabsProps {
  // The accessible name of the set, already in the person's language.
  readonly label: string;
  readonly lenses: readonly Lens[];
  // The one lens shown: an id from `lenses`.
  readonly current: string;
  readonly onPick: (id: string) => void;
  // What each lens shows, drawn inside the `role="tabpanel"` wrapper
  // and labelled by that lens's tab.
  readonly panel: Snippet<[Lens]>;
  // A count or a state beside a tab's name, usually a Badge. The
  // snippet is asked for every tab and answers for the ones it has
  // something to say about.
  readonly mark?: Snippet<[Lens]>;
}

// The part of a key event the tabs read, so the wiring is driven in a
// test by a plain object rather than a browser's event.
export type Pressed = Pick<KeyboardEvent, "key" | "preventDefault">;

// The one key every element's attachment travels under. A single key
// for the whole part keeps an attachment the same entry from one draw
// to the next, so a redraw does not let go of an element and take it
// again.
const HOLD = createAttachmentKey();

export interface ListWire {
  readonly role: "tablist";
  readonly "aria-label": string;
}

export interface TabWire {
  readonly role: "tab";
  readonly id: string;
  readonly "aria-controls": string;
  readonly "aria-selected": boolean;
  readonly tabindex: 0 | -1;
  readonly onclick: () => void;
  readonly onkeydown: (event: Pressed) => void;
  readonly [hold: symbol]: Attachment<HTMLElement>;
}

export interface PanelWire {
  readonly role: "tabpanel";
  readonly id: string;
  readonly "aria-labelledby": string;
  readonly hidden: boolean;
  // `0` only on the shown panel while nothing inside it takes the
  // focus, so a keyboard can still reach what it says (APG Tabs).
  readonly tabindex: 0 | undefined;
  // Present on the shown panel only: the seat watches that panel for
  // something the Tab key reaches.
  readonly [watch: symbol]: Attachment<HTMLElement>;
}

export interface TabLook {
  readonly lens: Lens;
  // The tab whose panel is shown; a look draws its mark from this.
  readonly current: boolean;
  readonly wire: TabWire;
}

export interface PanelLook {
  readonly lens: Lens;
  // The content mounts only while its lens is current, so a caller
  // may use mount as its data lifecycle; the wrapper is drawn for every
  // lens so each `aria-controls` resolves.
  readonly shown: boolean;
  readonly wire: PanelWire;
}

export interface TabsLook {
  readonly list: ListWire;
  readonly tabs: readonly TabLook[];
  readonly panels: readonly PanelLook[];
  readonly panel: Snippet<[Lens]>;
  readonly mark: Snippet<[Lens]> | undefined;
}

// What the seat lends the wiring: the elements it holds and the one
// fact it measures.
export interface Hands {
  // Puts the focus on the drawn tab of this lens.
  readonly focus: (id: string) => void;
  // Holds the drawn tab of this lens; the same attachment on every
  // draw for one id.
  readonly keep: (id: string) => Attachment<HTMLElement>;
  // Watches the shown panel for something the Tab key reaches.
  readonly watch: Attachment<HTMLElement>;
  // Whether the shown panel holds something the Tab key reaches, as
  // `watch` last measured it.
  readonly reachable: boolean;
}

// What the Tab key reaches inside a panel: a link, an enabled control,
// an editable region, or anything that put itself in the sequence.
// A panel holding none of them is a stop of its own.
export const REACHABLE = [
  "a[href]",
  "button:not([disabled])",
  "input:not([disabled]):not([type=hidden])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  "summary",
  "[contenteditable]:not([contenteditable=false])",
  "[tabindex]:not([tabindex='-1'])",
].join(", ");

// Where an arrow, Home or End lands among `total` tabs from the tab at
// `at` (-1 when the current id names no lens): the arrows wrap, and the
// ends are the ends. `undefined` is a key the tabs leave alone; Space
// and Enter are among them, because the switch already followed the
// focus and the click path answers them.
export function landing(total: number, at: number, key: string): number | undefined {
  const last = total - 1;
  if (total === 0) return undefined;
  switch (key) {
    case "ArrowRight":
      return at === last ? 0 : at + 1;
    case "ArrowLeft":
      return at <= 0 ? last : at - 1;
    case "Home":
      return 0;
    case "End":
      return last;
    default:
      return undefined;
  }
}

export function lookOf(props: TabsProps, uid: string, hands: Hands): TabsLook {
  const { label, lenses, current, onPick } = props;
  const at = lenses.findIndex((lens) => lens.id === current);
  const tabId = (lens: Lens): string => `${uid}-tab-${lens.id}`;
  const panelId = (lens: Lens): string => `${uid}-panel-${lens.id}`;

  const travel = (event: Pressed): void => {
    const to = landing(lenses.length, at, event.key);
    const lens = to === undefined ? undefined : lenses[to];
    if (lens === undefined) return;
    event.preventDefault();
    onPick(lens.id);
    hands.focus(lens.id);
  };

  // The one Tab stop: the current tab, or the first while the current
  // id names no lens, so the strip is never left without a way in.
  const stop = lenses[at === -1 ? 0 : at]?.id;

  const tabOf = (lens: Lens): TabLook => {
    const chosen = lens.id === current;
    return {
      lens,
      current: chosen,
      wire: {
        role: "tab",
        id: tabId(lens),
        "aria-controls": panelId(lens),
        "aria-selected": chosen,
        tabindex: lens.id === stop ? 0 : -1,
        onclick: () => {
          onPick(lens.id);
        },
        onkeydown: travel,
        [HOLD]: hands.keep(lens.id),
      },
    };
  };

  const panelOf = (lens: Lens): PanelLook => {
    const shown = lens.id === current;
    const wire: PanelWire = {
      role: "tabpanel",
      id: panelId(lens),
      "aria-labelledby": tabId(lens),
      hidden: !shown,
      tabindex: shown && !hands.reachable ? 0 : undefined,
    };
    return { lens, shown, wire: shown ? { ...wire, [HOLD]: hands.watch } : wire };
  };

  return {
    list: { role: "tablist", "aria-label": label },
    tabs: lenses.map(tabOf),
    panels: lenses.map(panelOf),
    panel: props.panel,
    mark: props.mark,
  };
}

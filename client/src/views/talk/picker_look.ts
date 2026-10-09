// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The model picker as the settings row draws it: the three-segment
// token, the popover's sections with their rows, the room each keeps,
// and what a pick does. `picker.ts` decides the rules and
// `picker_scene.ts` what one drawing reads; this builds the whole value
// `picker.look.svelte` is given, every role, key handler and `aria-*`
// value inside a wire bag (client D95). The seat (`settings_row.svelte`)
// holds what is open, typed and pointed at.

import type { Attachment } from "svelte/attachments";

import type { Lang } from "../../core/lang";
import { fill, say } from "../../core/lang";
import type { PopoverBinding, PopoverColumn, PopoverRow } from "../parts/popover";
import { HOLD } from "./pill";
import type { Combination, Entry, Offer, Section, Segment } from "./picker";
import { SECTION, appliedIn, matches, offersOf, orderOf, recentOf, roomOf, sameCombination, secondLevel, sectionOf, windowed } from "./picker";
import type { PickerFacts, PickerHeld, Scene } from "./picker_scene";
import { combinationOf, comboId, comboName, entryWords, levelWord, modelsSection, noMatch, sceneOf } from "./picker_scene";


// The row that opens a list in full. No entry id holds a NUL.
export const MORE = "\u0000more";

// What only the seat can do.
export interface PickerHands {
  readonly open: (segment: Segment | null) => void;
  readonly close: (focus: "token" | "stay") => void;
  readonly hold: (change: Partial<Pick<PickerHeld, "pick" | "parent" | "query" | "whole">>) => void;
  readonly keep: (combination: Combination) => void;
  readonly bound: (binding: PopoverBinding) => void;
  readonly cursor: (id: string | null) => void;
  readonly focusFilter: () => void;
  readonly holdToken: Attachment<HTMLElement>;
  readonly holdFilter: Attachment<HTMLElement>;
  readonly holdFrame: Attachment<HTMLElement>;
}

// One segment of the token, as drawn.
export interface TokenSegment {
  readonly segment: Segment;
  readonly label: string;
  // The picker is open on this segment's section.
  readonly open: boolean;
}

// The bag spread on the token. A press reads which segment was under
// the pointer from the drawn `data-segment`; a key press has none.
export interface TokenWire {
  readonly "aria-label": string;
  readonly "aria-haspopup": "dialog";
  readonly "aria-expanded": boolean;
  readonly onclick: (event: { readonly target: EventTarget | null }) => void;
  readonly [hold: symbol]: Attachment<HTMLElement>;
}

// How one section of the open picker is drawn beside the popover's own
// list value: its heading, the room it keeps, and the names its rows
// are heard by where those differ from their words.
export interface SectionLook {
  readonly heading: string;
  readonly aside: string | undefined;
  // Rows of room the list keeps below its rows, so its height is the
  // largest any reachable state draws.
  readonly spare: number;
  // The rows are laid across, as one band.
  readonly band: boolean;
  // The list is opened in full and scrolls inside its room.
  readonly scrolls: boolean;
  readonly empty: string;
  readonly heard: Readonly<Record<string, string>>;
}

// The bag spread on the filter box, which holds the focus while the
// picker is open and hands every menu key but its own to the popover.
export interface FilterWire {
  readonly type: "search";
  readonly "aria-label": string;
  readonly placeholder: string;
  readonly "aria-activedescendant": string | undefined;
  readonly "aria-controls": string | undefined;
  readonly value: string;
  readonly oninput: (event: { readonly currentTarget: { readonly value: string } }) => void;
  readonly onkeydown: (event: KeyboardEvent) => void;
  readonly [hold: symbol]: Attachment<HTMLElement>;
}

export interface PickerMenu {
  readonly columns: readonly PopoverColumn[];
  readonly sections: Readonly<Record<string, SectionLook>>;
  // The thinking band's room, kept while the selection has no band.
  readonly bandRoom: boolean;
  readonly filter: FilterWire;
  readonly onApply: (column: PopoverColumn, row: PopoverRow) => void;
  // Escape: closes onto the token.
  readonly onClose: () => void;
  // A press outside the token and the popover: closes where it landed.
  readonly onOutside: () => void;
  readonly bind: (binding: PopoverBinding) => void;
  readonly onCursorChange: (id: string | null) => void;
}

export interface PickerLook {
  // Spread on the box holding the token and the popover, whose edge an
  // outside press is measured against.
  readonly frame: Readonly<Record<symbol, Attachment<HTMLElement>>>;
  readonly token: TokenWire;
  readonly segments: readonly TokenSegment[];
  readonly menu: PickerMenu | undefined;
}

// The picker, or nothing when the city serves no model: an entry with
// no options is hidden rather than drawn empty.
export function pickerOf(lang: Lang, facts: PickerFacts, held: PickerHeld, hands: PickerHands): PickerLook | undefined {
  const scene = sceneOf(lang, facts, held);
  if (scene.offers.length === 0) return undefined;
  const segments = segmentsOf(scene, held);
  return {
    frame: { [HOLD]: hands.holdFrame },
    token: {
      "aria-label": segments.map((each) => each.label).join(" · "),
      "aria-haspopup": "dialog",
      "aria-expanded": held.open,
      onclick: (event) => {
        if (held.open) hands.close("token");
        else hands.open(segmentAt(event.target));
      },
      [HOLD]: hands.holdToken,
    },
    segments,
    menu: held.open ? menuOf(scene, facts, held, hands) : undefined,
  };
}

function segmentsOf(scene: Scene, held: PickerHeld): readonly TokenSegment[] {
  const { offer, used, lang } = scene;
  const open = (segment: Segment): boolean => held.open && held.segment === segment;
  if (offer === undefined) return [{ segment: "model", label: say(lang, "talk_no_model"), open: open("model") }];
  return [
    { segment: "model", label: offer.model, open: open("model") },
    { segment: "provider", label: offer.provider, open: open("provider") },
    ...(used === null ? [] : [{ segment: "level" as const, label: levelWord(scene, used.level), open: open("level") }]),
  ];
}

function segmentAt(target: EventTarget | null): Segment | null {
  const marked = target instanceof Element ? target.closest("[data-segment]")?.getAttribute("data-segment") : null;
  const segments: readonly Segment[] = ["model", "provider", "level"];
  return segments.find((each) => each === marked) ?? null;
}

function menuOf(scene: Scene, facts: PickerFacts, held: PickerHeld, hands: PickerHands): PickerMenu {
  const { lang, order, offers, offer, used, parent } = scene;
  const room = roomOf(order, offers);
  const now = combinationOf(scene);
  const recent = recentOf(held.opened, held.kept, offers);
  const typed = held.query.trim() !== "";
  const narrowed = (section: Section, entries: readonly Entry[]): readonly Entry[] =>
    typed && section === modelsSection(order) ? entries.filter((entry) => matches(entry, held.query)) : entries;
  const shown = (section: Section, entries: readonly Entry[], pinned: string | undefined) =>
    windowed(narrowed(section, entries), pinned, typed || held.whole.includes(section));
  const firstShown = shown(SECTION.first, scene.first, held.pinned.first);
  const second = secondLevel(order, parent);
  const secondShown = shown(SECTION.second, second, held.pinned.second);
  const levels = offer?.facts.thinking.levels ?? [];
  const thinking = levels.map((level) => ({
    id: level,
    label: used?.level === level && used.by === "default" ? fill(say(lang, "picker_effort_default"), { effort: levelWord(scene, level) }) : levelWord(scene, level),
    secondary: say(lang, `effort_note_${level}`),
    chosen: used?.level === level,
  }));
  const rowsOf = (section: Section, view: { entries: readonly Entry[]; more: number }, chosen: string | undefined): PopoverRow[] => [
    ...view.entries.map((entry) => ({ id: entry.id, ...entryWords(scene, section, entry), chosen: entry.id === chosen })),
    ...(view.more > 0 ? [{ id: MORE, label: fill(say(lang, "picker_more_count"), { n: String(view.more) }) }] : []),
  ];
  const columns: PopoverColumn[] = [
    ...(recent.length === 0
      ? []
      : [{ id: SECTION.recent, label: "picker_recent" as const, rows: recent.map((combo) => ({ id: comboId(combo), label: comboName(scene, combo), chosen: now !== null && sameCombination(combo, now) })) }]),
    { id: SECTION.first, label: order === "models_first" ? "talk_column_model" : "picker_providers", rows: rowsOf(SECTION.first, firstShown, parent?.id) },
    { id: SECTION.second, label: order === "models_first" ? "picker_providers" : "talk_column_model", rows: rowsOf(SECTION.second, secondShown, offer === undefined ? undefined : order === "models_first" ? offer.endpoint : offer.model) },
    ...(thinking.length === 0 ? [] : [{ id: SECTION.thinking, label: "talk_column_effort" as const, rows: thinking }]),
  ];
  const parentName = parent === undefined ? "" : entryWords(scene, SECTION.first, parent).label;
  const lineCount = (view: { entries: readonly Entry[]; more: number }): number => view.entries.length + (view.more > 0 ? 1 : 0);
  const sections: Record<string, SectionLook> = {
    [SECTION.recent]: { heading: say(lang, "picker_recent"), aside: undefined, spare: 0, band: false, scrolls: false, empty: "", heard: Object.fromEntries(recent.map((combo) => [comboId(combo), fill(say(lang, "picker_recent_name"), { choice: comboName(scene, combo) })])) },
    [SECTION.first]: { heading: say(lang, order === "models_first" ? "talk_column_model" : "picker_providers"), aside: undefined, spare: Math.max(0, room.first - lineCount(firstShown)), band: false, scrolls: firstShown.entries.length > room.first, empty: noMatch(lang, order, SECTION.first), heard: {} },
    [SECTION.second]: {
      heading: fill(say(lang, order === "models_first" ? "picker_providers_of" : "picker_models_at"), order === "models_first" ? { model: parentName } : { provider: parentName }),
      aside: say(lang, "picker_price_head"),
      spare: Math.max(0, room.second - lineCount(secondShown)),
      band: false,
      scrolls: secondShown.entries.length > room.second,
      empty: noMatch(lang, order, SECTION.second),
      heard: {},
    },
    [SECTION.thinking]: { heading: fill(say(lang, "picker_thinking_of"), { provider: offer?.provider ?? "" }), aside: undefined, spare: 0, band: true, scrolls: false, empty: "", heard: {} },
  };
  return {
    columns,
    sections,
    bandRoom: room.band && thinking.length === 0,
    filter: filterOf(scene, held, hands),
    onApply: (column, row) => {
      apply(scene, facts, held, hands, { section: sectionNamed(column.id), row, recent });
    },
    onClose: () => {
      keepNow(hands, now);
      hands.close("token");
    },
    onOutside: () => {
      keepNow(hands, now);
      hands.close("stay");
    },
    bind: hands.bound,
    onCursorChange: hands.cursor,
  };
}

function sectionNamed(id: string): Section {
  switch (id) {
    case SECTION.recent:
      return SECTION.recent;
    case SECTION.second:
      return SECTION.second;
    case SECTION.thinking:
      return SECTION.thinking;
    default:
      return SECTION.first;
  }
}

interface Pressed {
  readonly section: Section;
  readonly row: PopoverRow;
  readonly recent: readonly Combination[];
}

// What a row does. A "more…" row opens its list in full; a model or a
// provider changes the selection and leaves the picker open on the band
// under it; a level or a recent combination applies and closes.
function apply(scene: Scene, facts: PickerFacts, held: PickerHeld, hands: PickerHands, pressed: Pressed): void {
  const { section, row } = pressed;
  if (row.id === MORE) {
    hands.hold({ whole: [...held.whole, section] });
    hands.focusFilter();
    return;
  }
  switch (section) {
    case SECTION.first:
      chooseFirst(scene, facts, hands, row.id);
      break;
    case SECTION.second: {
      const offer = scene.parent?.offers.find((each) => (scene.order === "models_first" ? each.endpoint : each.model) === row.id);
      if (offer !== undefined) choose(facts, hands, offer);
      break;
    }
    case SECTION.thinking: {
      const level = scene.offer?.facts.thinking.levels.find((each) => each === row.id);
      if (level !== undefined) facts.pick.level(level);
      if (scene.offer !== undefined && level !== undefined) hands.keep({ endpoint: scene.offer.endpoint, model: scene.offer.model, level });
      break;
    }
    case SECTION.recent: {
      const combo = pressed.recent.find((each) => comboId(each) === row.id);
      if (combo === undefined) break;
      facts.pick.model({ endpoint: combo.endpoint, model: combo.model });
      if (combo.level !== null) facts.pick.level(combo.level);
      hands.keep(combo);
      break;
    }
  }
  if (appliedIn(section) === "closes") hands.close("token");
  else hands.focusFilter();
}

// A first-level row expands its second level. Under models first it is
// also a choice: the model at the provider in force when that provider
// serves it, else at its first provider.
function chooseFirst(scene: Scene, facts: PickerFacts, hands: PickerHands, id: string): void {
  hands.hold({ parent: id });
  if (scene.order === "providers_first") return;
  const entry = scene.first.find((each) => each.id === id);
  const offer = entry?.offers.find((each) => each.endpoint === scene.offer?.endpoint) ?? entry?.offers.at(0);
  if (offer !== undefined) choose(facts, hands, offer);
}

function choose(facts: PickerFacts, hands: PickerHands, offer: Offer): void {
  const names = { endpoint: offer.endpoint, model: offer.model };
  hands.hold({ pick: names });
  facts.pick.model(names);
}

function keepNow(hands: PickerHands, now: Combination | null): void {
  if (now !== null) hands.keep(now);
}

function filterOf(scene: Scene, held: PickerHeld, hands: PickerHands): FilterWire {
  const target = modelsSection(scene.order);
  return {
    type: "search",
    "aria-label": say(scene.lang, "talk_find_model"),
    placeholder: say(scene.lang, "talk_find_model"),
    "aria-activedescendant": held.active ?? undefined,
    "aria-controls": held.binding?.controls.join(" "),
    value: held.query,
    oninput: (event) => {
      hands.hold({ query: event.currentTarget.value });
      held.binding?.pointColumn(target);
    },
    onkeydown: (event) => {
      if (event.isComposing || event.key === "Home" || event.key === "End") return;
      if (held.binding?.keys(event) !== true) return;
      event.preventDefault();
      event.stopPropagation();
    },
    [HOLD]: hands.holdFilter,
  };
}

// The section a segment opens on, for the seat to point the cursor at.
export function startOf(facts: PickerFacts, segment: Segment | null): Section {
  return sectionOf(orderOf(offersOf(facts.endpoints)), segment);
}

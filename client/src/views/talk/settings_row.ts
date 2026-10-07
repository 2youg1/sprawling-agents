// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the settings row under the box decides before anything is drawn
// (docs/frontend-method.md §7I, client/Spec.lean §4-60): which providers
// the model entry offers, the two rows and the apart thinking list of its
// panel, what a pick in that panel does, and the whole value
// `settings_row.look.svelte` draws. The row is a seat and a look (client
// D95): `settings_row.svelte` holds which menu is open, the chosen
// provider, the typed filter and the drawn elements, and this module
// builds every role, key handler and `aria-*` value the look spreads.
// The permissions entry's half lives in `policy.ts`.

import type { Snippet } from "svelte";
import type { Attachment } from "svelte/attachments";

import type { Key, Lang } from "../../core/lang";
import { say } from "../../core/lang";
import type { PopoverBinding, PopoverColumn, PopoverRow } from "../parts/popover";
import type { Choice, Pill } from "./composer";
import { FILTER_AFTER, splitModel } from "./composer";
import { HOLD } from "./pill";
import type { TriggerWire } from "./pill";
import type { PermissionsLook } from "./policy";

// What the row draws: everything before a session starts, and only the
// notice that a sentence the link did not take was kept in the box once
// one has (`talk_not_live`).
export type RowDraws = "everything" | "notice";

// The menu open on the row. The workspace chip's menu is the pill's own.
export type RowMenu = "policy" | "model" | null;

// The menu a fixture draws open; every screen starts closed.
export type RowStarts = "open" | "closed" | "model";

export function menuOf(starts: RowStarts): RowMenu {
  switch (starts) {
    case "open":
      return "policy";
    case "model":
      return "model";
    case "closed":
      return null;
  }
}

// The columns of the model panel, named by what they hold.
export const COLUMN = { provider: "provider", model: "model", effort: "effort" } as const;

export interface Provider {
  readonly id: string;
  readonly label: string;
}

// Every provider the model pill offers a model of, once each, in the
// order its first model appears.
export function providersOf(models: Pill): readonly Provider[] {
  return models.choices
    .flatMap((row) => {
      const model = splitModel(row.value);
      return model === null ? [] : [{ id: model.endpoint, label: row.note ?? model.endpoint }];
    })
    .filter((row, index, all) => all.findIndex((each) => each.id === row.id) === index);
}

// The models one provider serves.
export function modelsFor(models: Pill, endpoint: string | undefined): readonly Choice[] {
  return models.choices.filter((row) => splitModel(row.value)?.endpoint === endpoint);
}

// The provider whose models the panel lists: the one the person pointed
// at, else the one serving the model in force, else the first.
export function parentOf(pointed: string | null, models: Pill): string | undefined {
  return pointed ?? splitModel(models.value ?? "")?.endpoint ?? providersOf(models).at(0)?.id;
}

// The model panel: provider and model are two steps of one table, so
// their lists stack as rows - provider first, then the models that
// provider serves, which grow the second row as the first is chosen.
// Thinking is not a step of that table: it is its own choice, listed
// apart under a rule (the person's design). The filter narrows the
// model row alone and never changes what is chosen.
export function modelColumns(models: Pill, effort: Pill, parent: string | undefined, query: string): readonly PopoverColumn[] {
  const needle = query.trim().toLowerCase();
  return [
    {
      id: COLUMN.provider,
      label: "talk_column_provider",
      rows: providersOf(models).map((row) => ({ ...row, chosen: row.id === parent })),
    },
    {
      id: COLUMN.model,
      label: "talk_column_model",
      rows: modelsFor(models, parent)
        .filter((row) => row.label.toLowerCase().includes(needle))
        .map((row) => ({ id: row.value, label: row.label, chosen: row.value === models.value })),
    },
    {
      id: COLUMN.effort,
      label: "talk_column_effort",
      apart: true,
      rows: effort.choices.map((row) => ({ id: row.value, label: row.label, secondary: row.note, chosen: row.value === effort.value })),
    },
  ];
}

// Which keys the search box keeps for its own text - Home and End move
// its text cursor, and a key that composes a character belongs to the
// input method - and which go to the popover's key table.
export function searchKeeps(key: string, composing: boolean): boolean {
  return composing || key === "Home" || key === "End";
}

// Whether a binding the popover hands over again is the one already
// held. The popover hands its binding over each time its props change,
// as a new object; holding each new object would change the row's look,
// which changes the popover's props again, without end.
export function sameBinding(held: PopoverBinding | null, next: PopoverBinding): boolean {
  return (
    held !== null &&
    held.keys === next.keys &&
    held.pointColumn === next.pointColumn &&
    held.controls.join(" ") === next.controls.join(" ")
  );
}

// What a closed pill shows: the label of the value in force, or its
// placeholder.
export function faceOf(pill: Pill): string {
  return pill.choices.find((row) => row.value === pill.value)?.label ?? pill.placeholder;
}

// The props of the model panel, which is the shared popover part; the
// look adds the search box and the row drawing, which are drawing.
export interface ModelMenu {
  readonly label: Key;
  readonly columns: readonly PopoverColumn[];
  readonly layout: "rows";
  readonly onApply: (column: PopoverColumn, row: PopoverRow) => void;
  readonly onClose: () => void;
  readonly bind: ((binding: PopoverBinding) => void) | undefined;
  readonly onCursorChange: (id: string | null) => void;
}

// The bag spread on the search box of a long model row. The box keeps
// the focus; Home and End stay with its text cursor, and every other
// menu key goes to the popover's key table through its binding.
export interface SearchWire {
  readonly type: "search";
  readonly "aria-label": string;
  readonly placeholder: string;
  readonly "aria-activedescendant": string | undefined;
  readonly "aria-controls": string | undefined;
  readonly value: string;
  readonly onfocus: () => void;
  readonly oninput: (event: { readonly currentTarget: { readonly value: string } }) => void;
  readonly onkeydown: (event: KeyboardEvent) => void;
  readonly [hold: symbol]: Attachment<HTMLElement>;
}

export interface ModelLook {
  readonly trigger: TriggerWire;
  readonly model: string;
  readonly effort: string;
  readonly menu: ModelMenu | undefined;
  readonly search: SearchWire | undefined;
}

// Everything the look is given.
export interface SettingsRowLook {
  // The workspace chip and the sandbox, drawn by their own seats.
  readonly facts: Snippet | undefined;
  // Said when a sentence the link did not take is kept in the box.
  readonly notLive: string | undefined;
  readonly model: ModelLook | undefined;
  readonly permissions: PermissionsLook | undefined;
}

// What the model entry is drawn from.
export interface ModelDrawing {
  readonly lang: Lang;
  readonly models: Pill;
  readonly effort: Pill;
}

// What the seat holds for the model entry between draws.
export interface ModelHeld {
  readonly open: boolean;
  readonly pointed: string | null;
  readonly query: string;
  readonly binding: PopoverBinding | null;
  readonly active: string | null;
}

// What only the seat can do.
export interface ModelHands {
  readonly toggle: () => void;
  readonly close: () => void;
  readonly point: (provider: string) => void;
  readonly query: (text: string) => void;
  readonly bound: (binding: PopoverBinding) => void;
  readonly cursor: (id: string | null) => void;
  readonly focus: { readonly trigger: () => void; readonly search: () => void };
  readonly hold: { readonly trigger: Attachment<HTMLElement>; readonly search: Attachment<HTMLElement> };
}

// The model entry, or nothing when no provider offers a model: an entry
// with no options is hidden rather than drawn empty.
export function modelOf(drawing: ModelDrawing, held: ModelHeld, hands: ModelHands): ModelLook | undefined {
  const { lang, models, effort } = drawing;
  if (providersOf(models).length === 0) return undefined;
  const parent = parentOf(held.pointed, models);
  const searched = modelsFor(models, parent).length > FILTER_AFTER;
  const model = faceOf(models);
  const thinking = faceOf(effort);
  // A provider pick keeps the focus in the panel: in the search box when
  // the new provider's row still has one, else on the entry, because the
  // box it was in is about to go.
  const apply = (column: PopoverColumn, row: PopoverRow): void => {
    switch (column.id) {
      case COLUMN.provider:
        if (searched && modelsFor(models, row.id).length <= FILTER_AFTER) hands.focus.trigger();
        else hands.focus.search();
        hands.point(row.id);
        return;
      case COLUMN.model:
        models.pick(row.id);
        hands.focus.search();
        return;
      case COLUMN.effort:
        effort.pick(row.id);
        hands.focus.search();
        return;
      default:
        return;
    }
  };
  return {
    trigger: {
      "aria-label": `${models.label}: ${model} | ${thinking}`,
      "aria-haspopup": "dialog",
      "aria-expanded": held.open,
      onclick: hands.toggle,
      [HOLD]: hands.hold.trigger,
    },
    model,
    effort: thinking,
    menu: held.open
      ? {
          label: "talk_column_model",
          columns: modelColumns(models, effort, parent, held.query),
          layout: "rows",
          onApply: apply,
          onClose: hands.close,
          bind: searched ? hands.bound : undefined,
          onCursorChange: hands.cursor,
        }
      : undefined,
    search:
      held.open && searched
        ? {
            type: "search",
            "aria-label": say(lang, "talk_find_model"),
            placeholder: say(lang, "talk_find_model"),
            "aria-activedescendant": held.active ?? undefined,
            "aria-controls": held.binding?.controls.join(" "),
            value: held.query,
            onfocus: () => {
              held.binding?.pointColumn(COLUMN.model);
            },
            oninput: (event) => {
              hands.query(event.currentTarget.value);
              held.binding?.pointColumn(COLUMN.model);
            },
            onkeydown: (event) => {
              if (searchKeeps(event.key, event.isComposing)) return;
              if (held.binding?.keys(event) !== true) return;
              event.preventDefault();
              event.stopPropagation();
            },
            [HOLD]: hands.hold.search,
          }
        : undefined,
  };
}

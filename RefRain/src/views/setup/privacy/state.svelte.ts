// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { untrack } from "svelte";
import { SvelteMap } from "svelte/reactivity";

import type {
  IdemKey,
  PrivacyAction,
  PrivacyAnswer,
  PrivacyControl,
  PrivacyResult,
} from "../../../wire";
import { IDLE, step, type Action, type Event, type Stage } from "./entry";
import { actionOf, outcomeOf, resultOf, type EntryModel } from "./page";

export interface Source {
  readonly answer: () => PrivacyAnswer | undefined;

  readonly send: (action: PrivacyAction) => { readonly idem: IdemKey; readonly sent: boolean };
  readonly refresh: () => void;
}

export interface Held {
  readonly stage: Stage<IdemKey>;

  readonly shown: PrivacyResult | null;
}

const BLANK: Held = { stage: IDLE, shown: null };

const PAUSE_MS = 2_000;

export interface PrivacyPage {
  readonly heldOf: (control: PrivacyControl) => Held;
  readonly press: (model: EntryModel, action: Action) => void;
  readonly cancel: (model: EntryModel) => void;

  readonly confirm: (model: EntryModel) => void;

  readonly restoreAll: (models: readonly EntryModel[]) => void;
}

export function privacyPage(source: Source): PrivacyPage {
  const held = new SvelteMap<PrivacyControl, Held>();

  const heldOf = (control: PrivacyControl): Held => held.get(control) ?? BLANK;

  function feed(model: EntryModel, event: Event<IdemKey>): void {
    const control = model.row.control;
    const before = heldOf(control);
    const offered = (action: Action) => model.offers.some((offer) => offer.action === action);
    held.set(control, { stage: step(offered, before.stage, event).stage, shown: before.shown });
  }

  function confirm(model: EntryModel): void {
    const stage = heldOf(model.row.control).stage;
    if (stage.kind !== "confirming") return;
    const offer = model.offers.find((each) => each.action === stage.action);
    if (offer === undefined) {
      feed(model, { kind: "cancel" });
      return;
    }
    const { idem, sent } = source.send(actionOf(model.row.control, offer));
    feed(model, { kind: "confirm", idem });
    if (sent) source.refresh();
    else feed(model, { kind: "lost", idem });
  }

  $effect(() => {
    const answer = source.answer();
    if (answer === undefined) return;
    const sending = untrack(() => [...held].filter(([, each]) => each.stage.kind === "sending"));
    let open = false;
    for (const [control, each] of sending) {
      if (each.stage.kind !== "sending") continue;
      const shown = outcomeOf(answer, each.stage.idem);
      const result = shown === null ? "running" : resultOf(shown);
      const moved = step(() => false, each.stage, {
        kind: "outcome",
        idem: each.stage.idem,
        result,
      });
      held.set(control, { stage: moved.stage, shown: shown ?? each.shown });
      open ||= moved.stage.kind === "sending";
    }
    if (!open) return;
    const timer = setTimeout(source.refresh, PAUSE_MS);
    return () => {
      clearTimeout(timer);
    };
  });

  return {
    heldOf,
    press: (model, action) => {
      feed(model, { kind: "press", action });
    },
    cancel: (model) => {
      feed(model, { kind: "cancel" });
    },
    confirm,
    restoreAll: (models) => {
      for (const model of models) {
        // wording-ok: the wire name of a privacy action, not a word for a reader
        feed(model, { kind: "press", action: "restore" });
        confirm(model);
      }
    },
  };
}

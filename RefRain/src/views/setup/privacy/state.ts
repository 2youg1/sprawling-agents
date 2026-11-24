// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { get, readonly, writable, type Readable } from "../../../core/store";

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
  readonly heldOf: (control: PrivacyControl, source?: ReadonlyMap<PrivacyControl, Held>) => Held;
  readonly press: (model: EntryModel, action: Action) => void;
  readonly cancel: (model: EntryModel) => void;

  readonly confirm: (model: EntryModel) => void;

  readonly restoreAll: (models: readonly EntryModel[]) => void;
}

export interface PrivacyPageOwner extends PrivacyPage {
  readonly state: Readable<ReadonlyMap<PrivacyControl, Held>>;
  readonly receive: (answer: PrivacyAnswer | undefined) => void;
  readonly close: () => void;
}

export function privacyPage(source: Source): PrivacyPageOwner {
  const held = writable<ReadonlyMap<PrivacyControl, Held>>(new Map());
  let timer: ReturnType<typeof setTimeout> | undefined;

  const heldOf = (control: PrivacyControl, snapshot = get(held)): Held =>
    snapshot.get(control) ?? BLANK;

  function feed(model: EntryModel, event: Event<IdemKey>): void {
    const control = model.row.control;
    const before = heldOf(control);
    const offered = (action: Action) => model.offers.some((offer) => offer.action === action);
    held.update((snapshot) =>
      new Map(snapshot).set(control, {
        stage: step(offered, before.stage, event).stage,
        shown: before.shown,
      }),
    );
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

  function receive(answer: PrivacyAnswer | undefined): void {
    clearTimeout(timer);
    if (answer === undefined) return;
    const snapshot = new Map(get(held));
    let open = false;
    let changed = false;
    for (const [control, each] of snapshot) {
      if (each.stage.kind !== "sending") continue;
      const shown = outcomeOf(answer, each.stage.idem);
      const result = shown === null ? "running" : resultOf(shown);
      const moved = step(() => false, each.stage, {
        kind: "outcome",
        idem: each.stage.idem,
        result,
      });
      snapshot.set(control, { stage: moved.stage, shown: shown ?? each.shown });
      open ||= moved.stage.kind === "sending";
      changed = true;
    }
    if (changed) held.set(snapshot);
    if (open) timer = setTimeout(source.refresh, PAUSE_MS);
  }

  return {
    state: readonly(held),
    receive,
    close: () => {
      clearTimeout(timer);
    },
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
        const restore = model.offers.find((offer) => offer.action === "restore");
        if (restore === undefined) continue;
        feed(model, { kind: "press", action: restore.action });
        confirm(model);
      }
    },
  };
}

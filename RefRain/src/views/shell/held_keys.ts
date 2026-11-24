// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { folded } from "../../core/keys";
import { HOLD_MS } from "../../core/press";

export function heldKeys(hands: {
  readonly eligible: () => boolean;
  readonly densityKey: () => string | undefined;
  readonly peek: (on: boolean) => void;
  readonly toggle: () => void;
}) {
  let density: ReturnType<typeof setTimeout> | null = null;
  let exposing: ReturnType<typeof setTimeout> | null = null;
  let long = false;
  const accelKey = (key: string): boolean => key === "Control" || key === "Meta";

  function expose(on: boolean): void {
    if (exposing !== null) clearTimeout(exposing);
    exposing = null;
    if (!on) delete document.documentElement.dataset.expose;
    else exposing = setTimeout(() => void (document.documentElement.dataset.expose = ""), HOLD_MS);
  }

  function cancelDensity(): void {
    if (density !== null) clearTimeout(density);
    density = null;
    hands.peek(false);
  }

  return {
    prepare(event: KeyboardEvent): void {
      expose(accelKey(event.key) && !event.isComposing && !event.repeat);
    },
    holding: () => density !== null,
    beginDensity(): void {
      if (density !== null || !hands.eligible()) return;
      long = false;
      density = setTimeout(() => {
        long = true;
        if (hands.eligible()) hands.peek(true);
      }, HOLD_MS);
    },
    cancelDensity,
    release: (event: KeyboardEvent): void => {
      if (accelKey(event.key)) expose(false);
      const tierKey = folded(event.key) === folded(hands.densityKey() ?? "");
      if (density === null || (!tierKey && (event.ctrlKey || event.metaKey))) return;
      cancelDensity();
      if (!long) hands.toggle();
    },
    forget: (): void => {
      expose(false);
      cancelDensity();
    },
  };
}

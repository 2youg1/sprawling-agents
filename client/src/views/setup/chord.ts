// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The control that records one shortcut in the keys section: a button
// that draws the chord in force, and once pressed listens for the next
// chord and binds it. What a key press means while it listens, and the
// bag the look spreads on the button, are decided here; the seat is
// `keys.svelte` and the look is `chord.look.svelte`.

import type { Action, Chord } from "../../core/keys";

// The keys that only change another key. A press of one of them alone
// is not a chord, so the control keeps listening.
const MODIFIERS: readonly string[] = ["Control", "Meta", "Shift", "Alt", "AltGraph", "CapsLock"];

// The part of a keyboard event this control reads.
export interface ChordPress {
  readonly key: string;
  readonly ctrlKey: boolean;
  readonly metaKey: boolean;
  readonly shiftKey: boolean;
  readonly preventDefault: () => void;
}

// What one press does while the control listens.
export type Heard =
  // A modifier alone: keep listening, and leave the press to the page.
  | { readonly kind: "modifier" }
  // Escape: stop listening and keep the chord in force.
  | { readonly kind: "cancel" }
  | { readonly kind: "chord"; readonly chord: Chord };

export function heard(press: ChordPress): Heard {
  if (MODIFIERS.includes(press.key)) return { kind: "modifier" };
  if (press.key === "Escape") return { kind: "cancel" };
  const accel = press.ctrlKey || press.metaKey;
  return { kind: "chord", chord: { accel, shift: accel && press.shiftKey, key: press.key } };
}

// The bag the look spreads on the one button. Enter and Space reach
// `onclick` through the platform, as every button's do (client/Spec.lean
// §7-2), so this control keeps no key table of its own beyond the
// listening one.
export interface ChordWire {
  readonly type: "button";
  readonly "aria-pressed": boolean;
  readonly onclick: () => void;
  readonly onblur: () => void;
  readonly onkeydown: (press: ChordPress) => void;
}

export interface ChordLook {
  readonly wire: ChordWire;
  // The action whose chord the button draws while it does not listen.
  readonly action: Action;
  // The chord in force, spelled; it changes exactly when the drawn
  // chord must be drawn again.
  readonly spelled: string;
  // The word the button draws in place of a chord, already in the
  // person's language: the prompt while it listens, the unbound word
  // while no chord reaches the action, `undefined` while a chord does.
  // Without it the button would be an empty box with no accessible name.
  readonly word: string | undefined;
  // The tooltip that names what the button does, already in the
  // person's language.
  readonly tip: string;
}

// What only the seat can do: it holds which action is listening and the
// keymap a chord is bound in.
export interface ChordHands {
  readonly listen: (action: Action | null) => void;
  readonly bind: (action: Action, chord: Chord) => void;
}

export interface ChordProps {
  readonly action: Action;
  readonly spelled: string;
  // Whether this action's control is the one listening.
  readonly listening: boolean;
  // The word drawn while it listens, already in the person's language.
  readonly prompt: string;
  // The word drawn while no chord reaches the action (`spelled` empty).
  readonly unbound: string;
  readonly tip: string;
}

export function chordOf(props: ChordProps, hands: ChordHands): ChordLook {
  const { action, listening } = props;
  return {
    action,
    spelled: props.spelled,
    word: wordOf(props),
    tip: props.tip,
    wire: {
      type: "button",
      "aria-pressed": listening,
      onclick: () => {
        hands.listen(action);
      },
      onblur: () => {
        hands.listen(null);
      },
      onkeydown: (press) => {
        if (!listening) return;
        const said = heard(press);
        switch (said.kind) {
          case "modifier":
            return;
          case "cancel":
            press.preventDefault();
            hands.listen(null);
            return;
          case "chord":
            press.preventDefault();
            hands.bind(action, said.chord);
            hands.listen(null);
            return;
        }
      },
    },
  };
}

function wordOf(props: ChordProps): string | undefined {
  if (props.listening) return props.prompt;
  return props.spelled === "" ? props.unbound : undefined;
}

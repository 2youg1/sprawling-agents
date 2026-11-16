// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import type { Action, Chord } from "../../core/keys";


const MODIFIERS: readonly string[] = ["Control", "Meta", "Shift", "Alt", "AltGraph", "CapsLock"];


export interface ChordPress {
  readonly key: string;
  readonly ctrlKey: boolean;
  readonly metaKey: boolean;
  readonly shiftKey: boolean;
  readonly preventDefault: () => void;
}


export type Heard =
  
  | { readonly kind: "modifier" }
  
  | { readonly kind: "cancel" }
  | { readonly kind: "chord"; readonly chord: Chord };

export function heard(press: ChordPress): Heard {
  if (MODIFIERS.includes(press.key)) return { kind: "modifier" };
  if (press.key === "Escape") return { kind: "cancel" };
  const accel = press.ctrlKey || press.metaKey;
  return { kind: "chord", chord: { accel, shift: accel && press.shiftKey, key: press.key } };
}


export interface ChordWire {
  readonly type: "button";
  readonly "aria-pressed": boolean;
  readonly onclick: () => void;
  readonly onblur: () => void;
  readonly onkeydown: (press: ChordPress) => void;
}

export interface ChordLook {
  readonly wire: ChordWire;
  
  readonly action: Action;
  
  
  readonly spelled: string;
  
  
  readonly word: string | undefined;
  
  
  readonly tip: string;
}


export interface ChordHands {
  readonly listen: (action: Action | null) => void;
  readonly bind: (action: Action, chord: Chord) => void;
}

export interface ChordProps {
  readonly action: Action;
  readonly spelled: string;
  
  readonly listening: boolean;
  
  readonly prompt: string;
  
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

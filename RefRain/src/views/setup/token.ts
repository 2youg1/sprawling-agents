// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One row of the colour page's token list: a colour well for the token,
// its name, and - while the person's override holds it - the way back to
// the colour the theme draws. The bags the look spreads are built here;
// the seat is `colours.svelte` and the look is `token.look.svelte`.

// The bag spread on the `<input type="color">`.
export interface WellWire {
  readonly type: "color";
  readonly "aria-label": string;
  // `#rrggbb`, the one form a colour well takes, or empty while the
  // engine has resolved nothing, which the well draws as black.
  readonly value: string;
  readonly onchange: (event: { readonly currentTarget: { readonly value: string } }) => void;
}

// The bag spread on the button that takes the override off.
export interface ResetWire {
  readonly type: "button";
  // Names the token, which the visible word alone does not.
  readonly "aria-label": string;
  readonly onclick: () => void;
}

export interface TokenLook {
  // The custom property's own name, never translated.
  readonly name: string;
  readonly well: WellWire;
  // Present while the person's override holds this token.
  readonly reset: { readonly word: string; readonly wire: ResetWire } | undefined;
}

export interface TokenProps {
  readonly name: string;
  // What the page draws for the token now, `#rrggbb`, or `undefined`
  // when the engine gave nothing that reads as a colour.
  readonly drawn: string | undefined;
  // The two words, already in the person's language: the visible one,
  // and the accessible name that says which token it resets.
  readonly reset: { readonly word: string; readonly label: string } | undefined;
}

// What only the seat can do: wear an override, or take it off.
export interface TokenHands {
  readonly pick: (name: string, value: string) => void;
  readonly unpick: (name: string) => void;
}

export function tokenOf(props: TokenProps, hands: TokenHands): TokenLook {
  const { name, reset } = props;
  return {
    name,
    well: {
      type: "color",
      "aria-label": name,
      value: props.drawn ?? "",
      onchange: (event) => {
        hands.pick(name, event.currentTarget.value);
      },
    },
    reset:
      reset === undefined
        ? undefined
        : {
            word: reset.word,
            wire: {
              type: "button",
              "aria-label": reset.label,
              onclick: () => {
                hands.unpick(name);
              },
            },
          },
  };
}

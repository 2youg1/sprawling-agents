// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export interface WellWire {
  readonly type: "color";
  readonly "aria-label": string;
  readonly value: string;
  readonly onchange: (event: { readonly currentTarget: { readonly value: string } }) => void;
}

export interface ResetWire {
  readonly type: "button";
  readonly "aria-label": string;
  readonly onclick: () => void;
}

export interface TokenLook {
  readonly name: string;
  readonly well: WellWire;
  readonly reset: { readonly word: string; readonly wire: ResetWire } | undefined;
}

export interface TokenProps {
  readonly name: string;
  readonly drawn: string | undefined;
  readonly reset: { readonly word: string; readonly label: string } | undefined;
}

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

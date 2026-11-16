// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import { Option, Schema } from "effect";

import { Address } from "../../wire";
import type { IconButtonProps } from "./icon_button";

export interface PathProps {
  
  readonly path: string;
  
  
  readonly base?: string | undefined;
  
  
  readonly onOpen?: (() => void) | undefined;
}


export interface PathWords {
  readonly reveal: string;
  readonly inert: string;
}


export interface OpenWire {
  readonly type: "button";
  readonly onclick: () => void;
}

export interface PathLook {
  readonly shown: string;
  
  readonly open: OpenWire | undefined;
  
  readonly reveal: IconButtonProps;
}

export function shownOf(path: string, base: string | undefined): string {
  return base !== undefined && path.startsWith(`${base}/`) ? path.slice(base.length + 1) : path;
}

export function lookOf(
  props: PathProps,
  words: PathWords,
  reveal: (address: Address) => void,
): PathLook {
  const { onOpen } = props;
  const address = Schema.decodeOption(Address)(props.path);
  return {
    shown: shownOf(props.path, props.base),
    open:
      onOpen === undefined
        ? undefined
        : {
            type: "button",
            onclick: () => {
              onOpen();
            },
          },
    reveal: Option.match(address, {
      onNone: () => ({ glyph: "reveal", label: words.reveal, why: words.inert }),
      onSome: (value) => ({
        glyph: "reveal",
        label: words.reveal,
        onPress: () => {
          reveal(value);
        },
      }),
    }),
  };
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Lang } from "../../core/lang";
import { say } from "../../core/lang";

export interface Option {
  readonly value: string;
  readonly label: string;
}

export interface NarrowingProps {
  readonly label: string;
  readonly options: readonly Option[];
  readonly current: string | null;
  readonly onPick: (value: string | null) => void;
}

export interface Changed {
  readonly currentTarget: { readonly value: string };
}

export interface ChoiceWire {
  readonly value: string;
  readonly onchange: (event: Changed) => void;
}

export interface NarrowingLook {
  readonly label: string;
  readonly options: readonly Option[];
  readonly wire: ChoiceWire;
}

const EVERY = "";

export function lookOf(props: NarrowingProps, lang: Lang): NarrowingLook {
  return {
    label: props.label,
    options: [{ value: EVERY, label: say(lang, "log_every") }, ...props.options],
    wire: {
      value: props.current ?? EVERY,
      onchange: (event) => {
        const value = event.currentTarget.value;
        props.onPick(value === EVERY ? null : value);
      },
    },
  };
}

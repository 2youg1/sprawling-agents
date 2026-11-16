// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One of the timeline's narrowing choices - a run, a log level, a
// module - decided (client D95: a seat, a look and this file). The
// first option is always "every one", which narrows nothing; it is the
// empty value, so no name a source could send is mistaken for it.

import type { Lang } from "../../core/lang";
import { say } from "../../core/lang";

export interface Option {
  readonly value: string;
  readonly label: string;
}

export interface NarrowingProps {
  // Already in the person's language.
  readonly label: string;
  readonly options: readonly Option[];
  // `null` is every one of them.
  readonly current: string | null;
  readonly onPick: (value: string | null) => void;
}

// What a change hands the wiring: the select it came from.
export interface Changed {
  readonly currentTarget: { readonly value: string };
}

// Spread on the `<select>`.
export interface ChoiceWire {
  readonly value: string;
  readonly onchange: (event: Changed) => void;
}

export interface NarrowingLook {
  readonly label: string;
  // The options in order, "every one" first.
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

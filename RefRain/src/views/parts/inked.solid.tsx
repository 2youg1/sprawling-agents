// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createSignal } from "solid-js";
import { painted, type Piece } from "./code";
import Look from "./inked.look.solid";

export interface InkedProps {
  readonly text: string;
  readonly source: string;
}

interface Answer extends InkedProps {
  readonly pieces: readonly Piece[];
}

export default function Inked(props: InkedProps) {
  const [answer, setAnswer] = createSignal<Answer | null>(null);
  const parts = (): readonly Piece[] => {
    const held = answer();
    if (held?.text === props.text && held.source === props.source) return held.pieces;
    return [{ ink: "plain", text: props.text }];
  };
  createEffect(
    () => [props.text, props.source] as const,
    ([text, source]) => {
      let active = true;
      void painted(text, source).then((pieces) => {
        if (active) setAnswer({ text, source, pieces });
      });
      return () => {
        active = false;
      };
    },
  );
  return <Look pieces={parts()} />;
}

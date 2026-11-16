// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Snippet } from "svelte";

import type { Key } from "../../core/lang";

export interface EmptyStateProps {
  readonly missing: Key;

  readonly shape?: Snippet;

  readonly action?: Snippet;

  readonly seat?: EmptySeat;
}

export type EmptySeat = "centred" | "region" | "inset";

export interface ShapeWire {
  readonly "aria-hidden": "true";
}

export interface EmptyLook {
  readonly seat: EmptySeat;
  readonly shapeWire: ShapeWire;

  readonly shape: Snippet | undefined;

  readonly sentence: string;
  readonly action: Snippet | undefined;
}

export function lookOf(props: EmptyStateProps, sentence: string): EmptyLook {
  return {
    seat: props.seat ?? "centred",
    shapeWire: { "aria-hidden": "true" },
    shape: props.shape,
    sentence,
    action: props.action,
  };
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Snippet } from "../../core/snippet";

import type { Key } from "../../core/lang";

export interface EmptyStateProps<Content = Snippet> {
  readonly missing: Key;

  readonly shape?: Content;

  readonly action?: Content;

  readonly seat?: EmptySeat;
}

export type EmptySeat = "centred" | "region" | "inset";

export interface ShapeWire {
  readonly "aria-hidden": "true";
}

export interface EmptyLook<Content = Snippet> {
  readonly seat: EmptySeat;
  readonly shapeWire: ShapeWire;

  readonly shape: Content | undefined;

  readonly sentence: string;
  readonly action: Content | undefined;
}

export function lookOf<Content>(
  props: EmptyStateProps<Content>,
  sentence: string,
): EmptyLook<Content> {
  return {
    seat: props.seat ?? "centred",
    shapeWire: { "aria-hidden": "true" },
    shape: props.shape,
    sentence,
    action: props.action,
  };
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { dynamic, type JSX } from "@solidjs/web";

export type Snippet<Args extends readonly unknown[] = readonly []> = (props: {
  readonly args: Args;
}) => JSX.Element;

interface RenderProps<Args extends readonly unknown[]> {
  readonly snippet: Snippet<Args> | undefined | null;
  readonly args: Args;
}

export function RenderSnippet<Args extends readonly unknown[]>(props: RenderProps<Args>) {
  const Rendered = dynamic(() => props.snippet);
  return <Rendered args={props.args} />;
}

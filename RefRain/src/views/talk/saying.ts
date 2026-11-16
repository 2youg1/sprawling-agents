// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import type { Snippet } from "svelte";

export interface SayingLook {
  readonly who: string;
  
  readonly laid: Snippet;
  
  
  readonly settled: string;
  readonly edge: string;
}


const EDGE = 10;


export function faded(open: string): Pick<SayingLook, "settled" | "edge"> {
  return { settled: open.slice(0, -EDGE), edge: open.slice(-EDGE) };
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { SliderLook } from "./slider";

export default function SliderView(props: SliderLook) {
  const bound = () => {
    const { oninput, ...wire } = props.wire;
    return { ...wire, onInput: oninput };
  };

  return (
    <>
      <div class={"flex h-control items-center gap-base"}>
        <input {...bound()} class={"min-w-0 flex-1 accent-accent"} />
        <span class={"figure w-figure text-right text-body text-text"}>{props.shown}</span>
      </div>
    </>
  );
}

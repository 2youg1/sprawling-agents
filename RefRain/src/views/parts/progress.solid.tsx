// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo } from "solid-js";
import Look from "./progress.look.solid";
import { lookOf } from "./progress";
import type { ProgressProps } from "./progress";

export default function ProgressView(props: ProgressProps) {
  const look = createMemo(() => lookOf(props));
  return (
    <>
      <Look {...look()} />
    </>
  );
}

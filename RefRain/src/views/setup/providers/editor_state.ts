// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createSignal } from "solid-js";

export function editorState<T extends object>(initial: T): T {
  const fields = { ...initial };
  for (const key in initial) {
    if (!Object.hasOwn(initial, key)) continue;
    const [field, setField] = createSignal(() => initial[key]);
    Object.defineProperty(fields, key, {
      enumerable: true,
      get: field,
      set: (value: T[typeof key]) => {
        setField(() => value);
      },
    });
  }
  return fields;
}

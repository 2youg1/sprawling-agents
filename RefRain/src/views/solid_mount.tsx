// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { render } from "@solidjs/web";
import { createSignal } from "solid-js";
import { scopedUi, UiScope } from "../ui";

export interface SolidMount<Props> {
  readonly update: (next: Props) => void;
  readonly dispose: () => void;
}

export function mountSolid<Props extends object>(
  target: HTMLElement,
  View: (props: Props) => JSX.Element,
  initial: Props,
): SolidMount<Props> {
  const [current, setCurrent] = createSignal({ props: initial });
  const scope = scopedUi();
  const dispose = render(
    () =>
      scope === undefined ? (
        <View {...current().props} />
      ) : (
        <UiScope value={scope}>
          <View {...current().props} />
        </UiScope>
      ),
    target,
  );
  return {
    update: (next) => {
      setCurrent({ props: next });
    },
    dispose,
  };
}

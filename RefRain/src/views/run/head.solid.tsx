// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo } from "solid-js";
import { ui } from "../../ui";
import { lookOf } from "./head";
import type { HeadProps } from "./head";
import Look from "./head.look.solid";
import { storeValue } from "../../core/solid_store";

export default function View(props: HeadProps) {
  const lang = ui().lang;
  const $lang = storeValue(lang);
  const look = createMemo(() => lookOf(props, $lang()));
  return (
    <>
      <Look {...look()} />
    </>
  );
}

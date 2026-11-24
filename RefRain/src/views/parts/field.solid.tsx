// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createUniqueId } from "solid-js";
export type { FieldProps } from "./field";
import { lookOf, type FieldProps } from "./field";
import Look from "./field.look.solid";

export default function FieldView(props: FieldProps) {
  const uid = createUniqueId();
  const look = createMemo(() => lookOf(props, uid));
  return (
    <>
      <Look {...look()} />
    </>
  );
}

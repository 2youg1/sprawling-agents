// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, Show } from "solid-js";
import { badgeOf } from "./badge";
import type { AnyBadgeProps } from "./badge";
import Look from "./badge.look.solid";

export default function BadgeView(props: AnyBadgeProps) {
  const look = createMemo(() => badgeOf(props));
  return <Show when={look()}>{(shown) => <Look {...shown()} />}</Show>;
}

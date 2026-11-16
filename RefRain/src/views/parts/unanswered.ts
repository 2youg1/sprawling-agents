// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Query } from "../../wire";

export interface UnansweredProps {
  readonly query: string;
  readonly reason?: string | null | undefined;
  readonly asked: Query;
}

export interface UnansweredLook {
  readonly said: string;
  readonly reason: string | undefined;
  readonly retry: { readonly label: string; readonly press: () => void };
}

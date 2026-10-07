// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Whether the way past a refusal goes through the model settings: no
// model chosen, a key missing, or a provider that turned the call down.
// One answer, so the refusal inside a turn and the one above the box
// offer the same next step.

import type { AxError } from "../../wire";

// The codes whose way out is the settings page.
const SETTLED_THERE: ReadonlySet<string> = new Set([
  "E_MODEL_UNCHOSEN",
  "E_CREDENTIAL_MISSING",
  "E_PROVIDER",
  "E_PROVIDER_ACCOUNTS_EXHAUSTED",
  "E_ENDPOINT_DIALECT_UNSUPPORTED",
]);

export function settledBySettings(error: AxError): boolean {
  return SETTLED_THERE.has(error.code) || (error.provider !== undefined && error.provider !== null);
}

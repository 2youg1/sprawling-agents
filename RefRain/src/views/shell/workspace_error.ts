// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { AxError } from "../../wire";

export function errorText(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function workspaceDiagnostic(diagnostic: AxError): string {
  const { code, action, subject, retry, recovery } = diagnostic;
  return [
    `[${code}]`,
    `operation = ${JSON.stringify(action)}`,
    `subject = ${JSON.stringify(subject)}`,
    `retry = ${JSON.stringify(retry)}`,
    `recovery = ${JSON.stringify(recovery)}`,
  ].join("\n");
}

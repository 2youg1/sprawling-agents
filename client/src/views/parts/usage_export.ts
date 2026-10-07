// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The two export buttons of a usage panel. The city writes the rows
// (`Query::UsageExport`, wire D33) so the columns have one author; the
// page asks again on every press, because a held answer would hand the
// person the uses as they stood when the panel first opened, and gives
// the text to the browser's own download. Nothing is written in the
// city's folder.

import type { Answer, ExportFormat } from "../../wire";

export const FORMATS: readonly ExportFormat[] = ["jsonl", "csv"];

export const TYPES: Record<ExportFormat, string> = {
  jsonl: "application/x-ndjson",
  csv: "text/csv",
};

// What one fresh answer to an export question comes to: the file's
// text, or the city saying it could not look - which ends the wait
// rather than leaving the button busy for good.
export type Exported =
  | { readonly kind: "file"; readonly body: string }
  | { readonly kind: "unavailable"; readonly query: string; readonly reason: string | undefined };

export function exportedOf(answer: Answer): Exported | undefined {
  if ("usage_export" in answer) return { kind: "file", body: answer.usage_export.body };
  if ("unavailable" in answer) {
    return { kind: "unavailable", query: answer.unavailable.query, reason: answer.unavailable.reason ?? undefined };
  }
  return undefined;
}

// What a look of the export buttons is given.
export interface UsageExportLook {
  readonly buttons: readonly {
    readonly format: ExportFormat;
    readonly label: string;
    readonly loading: boolean;
    readonly press: () => void;
  }[];
  // The last press the city could not answer, said in the reader's
  // language with the city's reason in its own words, or `undefined`.
  readonly unavailable: { readonly said: string; readonly reason: string | undefined } | undefined;
}

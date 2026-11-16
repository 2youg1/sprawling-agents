// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Answer, ExportFormat } from "../../wire";

export const FORMATS: readonly ExportFormat[] = ["jsonl", "csv"];

export const TYPES: Record<ExportFormat, string> = {
  jsonl: "application/x-ndjson",
  csv: "text/csv",
};

export type Exported =
  | { readonly kind: "file"; readonly body: string }
  | { readonly kind: "unavailable"; readonly query: string; readonly reason: string | undefined };

export function exportedOf(answer: Answer): Exported | undefined {
  if ("usage_export" in answer) return { kind: "file", body: answer.usage_export.body };
  if ("unavailable" in answer) {
    return {
      kind: "unavailable",
      query: answer.unavailable.query,
      reason: answer.unavailable.reason ?? undefined,
    };
  }
  return undefined;
}

export interface UsageExportLook {
  readonly buttons: readonly {
    readonly format: ExportFormat;
    readonly label: string;
    readonly loading: boolean;
    readonly press: () => void;
  }[];
  readonly unavailable: { readonly said: string; readonly reason: string | undefined } | undefined;
}

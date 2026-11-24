// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { say } from "../../core/lang";
import type { Key, Lang } from "../../core/lang";
import { WorkspaceError } from "../../core/workspace";

type WriteState = "clean" | "saving" | "saved" | "conflict" | "pending" | "refused";

export interface WriteReceipt {
  readonly kind: WriteState;
  readonly detail?: string;
  readonly diagnostic?: string;
}

const RECEIPT: Readonly<Record<Exclude<WriteState, "clean">, Key>> = {
  saving: "refrain_receipt_saving",
  saved: "refrain_receipt_saved",
  conflict: "refrain_receipt_conflict",
  pending: "refrain_receipt_pending",
  refused: "refrain_receipt_refused",
};

export function writeReceiptWord(receipt: WriteReceipt, dirty: boolean, lang: Lang): string {
  const { kind } = receipt;
  if (dirty && (kind === "clean" || kind === "saved")) return say(lang, "refrain_receipt_draft");
  return kind === "clean" ? "" : say(lang, RECEIPT[kind]);
}

export function writeRefusal(error: unknown, version: string): WriteReceipt {
  if (!(error instanceof WorkspaceError)) return { kind: "refused", detail: String(error) };
  const { code, action, subject, retry, recovery } = error.diagnostic;
  const kind =
    code === "E_VERSION_CONFLICT"
      ? "conflict"
      : code === "E_TOOL_UNAVAILABLE"
        ? "pending"
        : "refused";
  const diagnostic = Object.entries({ code, action, subject, retry, recovery, version })
    .map(([name, value]) => `${name} = ${JSON.stringify(value)}`)
    .join("\n");
  return { kind, detail: error.message, diagnostic };
}

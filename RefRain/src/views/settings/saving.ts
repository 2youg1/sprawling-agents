// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { AxCode, AxError } from "../../wire";

export type Saving =
  | { readonly kind: "held" }
  | { readonly kind: "draft" }
  | { readonly kind: "saving"; readonly from: string }
  | { readonly kind: "saved" }
  | { readonly kind: "refused"; readonly error: AxError }
  | { readonly kind: "unverified"; readonly from: string };

export const HELD: Saving = { kind: "held" };

export const RECEIPT_MS = 15_000;

const WRITE_REFUSED: ReadonlySet<AxCode> = new Set<AxCode>([
  "E_CONFIG_INVALID",
  "E_VERSION_CONFLICT",
  "E_STORAGE_FATAL",
]);

export function edited(changed: boolean): Saving {
  return changed ? { kind: "draft" } : HELD;
}

export function sent(from: string): Saving {
  return { kind: "saving", from };
}

export function answered(saving: Saving, version: string): Saving {
  switch (saving.kind) {
    case "saving":
    case "unverified":
      return version === saving.from ? saving : { kind: "saved" };
    case "held":
    case "draft":
    case "saved":
    case "refused":
      return saving;
  }
}

export function refused(saving: Saving, error: AxError): Saving {
  switch (saving.kind) {
    case "saving":
    case "unverified":
      return WRITE_REFUSED.has(error.code) ? { kind: "refused", error } : saving;
    case "held":
    case "draft":
    case "saved":
    case "refused":
      return saving;
  }
}

export function waited(saving: Saving): Saving {
  return saving.kind === "saving" ? { kind: "unverified", from: saving.from } : saving;
}

export interface Slot {
  readonly now: () => Saving;
  readonly mark: (saving: Saving) => void;
}

export function awaitReceipt(mine: Saving, slot: Slot): void {
  setTimeout(() => {
    if (slot.now() === mine) slot.mark(waited(mine));
  }, RECEIPT_MS);
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Where one save of a settings card stands (refrain roadmap §3-14,
// client-SPEC 7L). **Only the city's receipt says saved**: sending the
// frame says saving, the city answering again with a version other than
// the one the save was made against says saved, a refusal of the write
// says refused with the draft kept, and silence past the page's patience
// says unverified - the frame may have landed, and the next answer that
// moves the version still settles it as saved. A card whose text has
// not moved from what the city holds is held, and one edited and not
// sent is a draft.

import type { AxCode, AxError } from "../../wire";

export type Saving =
  | { readonly kind: "held" }
  | { readonly kind: "draft" }
  | { readonly kind: "saving"; readonly from: string }
  | { readonly kind: "saved" }
  | { readonly kind: "refused"; readonly error: AxError }
  | { readonly kind: "unverified"; readonly from: string };

export const HELD: Saving = { kind: "held" };

// How long a sent save waits for its receipt before it says it cannot
// tell: the page's own patience for an answer (`core/asking.ts`).
export const RECEIPT_MS = 15_000;

// The refusals a write of one document answers with: a name outside its
// domain or an area that does not read, a document that moved since the
// card read it, and a document the city cannot write.
const WRITE_REFUSED: ReadonlySet<AxCode> = new Set<AxCode>(["E_CONFIG_INVALID", "E_VERSION_CONFLICT", "E_STORAGE_FATAL"]);

// The person changed the card's text: a draft until it is sent, unless
// the text is what the city holds again.
export function edited(changed: boolean): Saving {
  return changed ? { kind: "draft" } : HELD;
}

// The frame left, made against the version the card read.
export function sent(from: string): Saving {
  return { kind: "saving", from };
}

// The city answered again with `version`.
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

// A refusal arrived while the card waited.
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

// The page's patience ran out before any receipt.
export function waited(saving: Saving): Saving {
  return saving.kind === "saving" ? { kind: "unverified", from: saving.from } : saving;
}

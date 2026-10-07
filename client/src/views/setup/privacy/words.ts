// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The privacy page's words for what the host's answer names. The host
// sends names only (`crates/wire/src/privacy.rs`); every sentence about a
// control, an edition or a reason is a phrase in `lang.json` keyed by
// that name. Each key below is spelled from a closed set of the wire, so
// a member the table has no phrase for fails the typecheck rather than
// reaching the page as a raw key.

import { isKey, type Key } from "../../../core/lang";
import type {
  PrivacyBuildEffect,
  PrivacyCategory,
  PrivacyControl,
  PrivacyEdition,
  PrivacyEditionFit,
  PrivacyNotWritten,
  PrivacyOriginal,
} from "../../../wire";

// What the page says about each control, one phrase per field.
export type ControlField = "title" | "does" | "benefit" | "affects" | "overlooked" | "undo" | "editions";

export function controlWord(control: PrivacyControl, field: ControlField): Key {
  const key: `privacy_control_${PrivacyControl}_${ControlField}` = `privacy_control_${control}_${field}`;
  return key;
}

export function categoryWord(category: PrivacyCategory): Key {
  const key: `privacy_category_${PrivacyCategory}` = `privacy_category_${category}`;
  return key;
}

export function editionWord(edition: PrivacyEdition): Key {
  const key: `privacy_edition_${PrivacyEdition}` = `privacy_edition_${edition}`;
  return key;
}

export function effectWord(effect: PrivacyBuildEffect): Key {
  const key: `privacy_effect_${PrivacyBuildEffect}` = `privacy_effect_${effect}`;
  return key;
}

export function fitWord(fit: PrivacyEditionFit): Key {
  const key: `privacy_fit_${PrivacyEditionFit}` = `privacy_fit_${fit}`;
  return key;
}

export function reasonWord(reason: PrivacyNotWritten): Key {
  const key: `privacy_reason_${PrivacyNotWritten}` = `privacy_reason_${reason}`;
  return key;
}

// The research's explanation of one line the host does not write. Only
// the lines not written have one, and which those are is the host's
// answer, so the key is looked up rather than spelled; the binary's
// `privacy::originals` test holds the table to exactly those lines.
export function originalReason(item: PrivacyOriginal): Key | null {
  const key = `privacy_original_${item}_reason`;
  return isKey(key) ? key : null;
}

// The facts the page states above every entry, in the order it states them.
export const PAGE_FACTS: readonly Key[] = [
  "privacy_page_fact_1",
  "privacy_page_fact_2",
  "privacy_page_fact_3",
  "privacy_page_fact_4",
  "privacy_page_fact_5",
  "privacy_page_fact_6",
];

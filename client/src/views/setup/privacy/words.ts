// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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

export type ControlField =
  "title" | "does" | "benefit" | "affects" | "overlooked" | "undo" | "editions";

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

export function originalReason(item: PrivacyOriginal): Key | null {
  const key = `privacy_original_${item}_reason`;
  return isKey(key) ? key : null;
}

export const PAGE_FACTS: readonly Key[] = [
  "privacy_page_fact_1",
  "privacy_page_fact_2",
  "privacy_page_fact_3",
  "privacy_page_fact_4",
  "privacy_page_fact_5",
  "privacy_page_fact_6",
];

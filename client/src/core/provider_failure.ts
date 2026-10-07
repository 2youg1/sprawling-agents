// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The way out a person reads when a model call failed. The city names
// the kind of failure; the words are this client's, one entry per kind in
// `lang.json`, and the city's own English sentence stays in the fold.
// Whether a refusal is worth asking again is the city's decision, carried
// as `retry`; the words follow it rather than judging the status here.

import type { ProviderFailureKind, Retry } from "../wire";
import { fill, say, type Lang } from "./lang";

export function providerClause(lang: Lang, failure: ProviderFailureKind, retry: Retry): string {
  switch (failure.kind) {
    case "exchange":
      return say(lang, "provider_exchange");
    case "cut":
      return say(lang, "provider_cut");
    case "silence":
      return say(lang, "provider_silence");
    case "refused":
      return fill(say(lang, retry === "yes" ? "provider_refused_busy" : "provider_refused"), {
        status: String(failure.status),
      });
    case "quota":
      return fill(say(lang, "provider_quota"), { status: String(failure.status) });
    case "overflow":
      return fill(say(lang, "provider_overflow"), { status: String(failure.status) });
    case "reported":
      return say(lang, "provider_reported");
    case "unreadable":
      return say(lang, "provider_unreadable");
    case "unbuilt":
      return say(lang, "provider_unbuilt");
  }
}

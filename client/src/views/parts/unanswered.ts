// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What stands where a view's answer would be when the city said it
// could not look (`Answer::Unavailable`). It names the question as the
// city spelled it back and offers the one recovery a page has: asking
// again, which a city that has since caught up answers.

import type { Query } from "../../wire";

export interface UnansweredProps {
  // The question as the city spelled it back.
  readonly query: string;
  // Why the city could not look, when it says (`Answer::Unavailable`,
  // wire D47); `null` where it only names the question.
  readonly reason?: string | null | undefined;
  // The question this view asked, sent again by the recovery.
  readonly asked: Query;
}

// What a look of the unanswered question is given.
export interface UnansweredLook {
  // The sentence naming the question, in the reader's language.
  readonly said: string;
  // The city's own reason, in its words, or `undefined`.
  readonly reason: string | undefined;
  readonly retry: { readonly label: string; readonly press: () => void };
}

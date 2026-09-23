// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// How a call is named wherever one is named: in the fold `calls.svelte`
// draws, in the posture line while the call is still running, and in the
// fork picker's second column. One spelling, so a person does not have
// to recognise the same call twice.

export function callWord(tool: string, subject: string | null | undefined): string {
  return subject === null || subject === undefined || subject === "" ? tool : `${tool} ${subject}`;
}

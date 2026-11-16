// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A run's identity as a link or a file name writes it, read by the
// check the wire generates.
//
// The city writes one spelling - the hyphenated lower-case uuid - and
// `kernel::RunId::parse` accepts that spelling and no other. This file
// carries no grammar of its own: it decodes with the generated schema,
// so a fragment or a transcript name holding bare hex, braces, a
// `urn:uuid:` prefix or upper case answers `None` here exactly as the
// city refuses it.

import { Schema } from "effect";
import type { Option } from "effect";

import { RunId as written } from "../wire";
import type { RunId } from "../wire";

// The one reader of a run id written into a link. Decoding rather than
// matching, because the pattern belongs to the schema the Rust wire
// generates; a second regular expression here would be a second answer
// to which ids exist.
const read = Schema.decodeOption(written);

export function readRunId(raw: string): Option.Option<RunId> {
  return read(raw);
}

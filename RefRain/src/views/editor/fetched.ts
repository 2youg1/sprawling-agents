// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { readAnswer } from "../../core/answered";
import type { Asking } from "../../core/asking";
import {
  fetchedOf,
  fetching,
  joinedBytes,
  nextBytes,
  type Fetching,
} from "../../core/document_bytes";
import { derived } from "../../core/store";
import type { B3Hash } from "../../wire";
import { pages } from "./pages";

const current = (fetched: { value: Fetching | null; lost: string | null }) => ({
  value: fetched.value === null ? null : fetchedOf(fetched.value),
  lost: fetched.lost,
});

export class StoredBytes {
  private readonly reader;
  readonly state;

  constructor(asking: Asking) {
    this.reader = pages(asking, {
      query: (at: Fetching) => {
        const range = nextBytes(at);
        return range === null ? null : { bytes: { version: at.version, range } };
      },
      read: (answer) => readAnswer(answer, (held) => ("bytes" in held ? held.bytes : undefined)),
      append: joinedBytes,
    });
    this.state = derived(this.reader.state, current);
  }

  get value() {
    return current(this.reader.current()).value;
  }
  get lost() {
    return this.reader.current().lost;
  }
  start(version: B3Hash | null): void {
    if (version !== (this.reader.current().value?.version ?? null)) {
      this.reader.start(version === null ? null : fetching(version));
    }
  }
  close(): void {
    this.reader.close();
  }
}

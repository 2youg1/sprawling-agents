// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { readAnswer } from "../../core/answered";
import type { Asking } from "../../core/asking";
import { joined, nextSpan, type Gathering } from "../../core/document_windows";
import { pages } from "./pages";

export class Gathered {
  private readonly reader;

  constructor(asking: Asking) {
    this.reader = pages(asking, {
      query: (at: Gathering) => {
        const range = nextSpan(at);
        return range === null ? null : { range: { version: at.version, range } };
      },
      read: (answer) => readAnswer(answer, (held) => ("range" in held ? held.range : undefined)),
      append: joined,
    });
  }

  get state() {
    return this.reader.state;
  }
  get value() {
    return this.reader.current().value;
  }
  get lost() {
    return this.reader.current().lost;
  }
  start(from: Gathering | null): void {
    this.reader.start(from);
  }
  close(): void {
    this.reader.close();
  }
}

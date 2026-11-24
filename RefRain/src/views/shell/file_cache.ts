// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { WorkspaceRecord } from "../../core/workspace";

export interface OpenedFile {
  readonly workspace: string;
  readonly record: WorkspaceRecord;
  readonly bytes: Uint8Array<ArrayBuffer>;
}

const KEPT_FILES = 3;
const KEPT_BYTES = 32 * 1024 * 1024;

export function cacheKey(workspace: string, record: string): string {
  return JSON.stringify([workspace, record]);
}

export class FileCache {
  readonly #held = new Map<string, OpenedFile>();
  #size = 0;

  get(workspace: string, id: string): OpenedFile | undefined {
    return this.#held.get(cacheKey(workspace, id));
  }

  delete(workspace: string, id: string): void {
    this.#remove(cacheKey(workspace, id));
  }

  keep(file: OpenedFile): void {
    const key = cacheKey(file.workspace, file.record.id);
    this.#remove(key);
    if (file.bytes.byteLength > KEPT_BYTES) return;
    this.#held.set(key, file);
    this.#size += file.bytes.byteLength;
    for (const oldest of this.#held.keys()) {
      if (this.#held.size <= KEPT_FILES && this.#size <= KEPT_BYTES) break;
      this.#remove(oldest);
    }
  }

  #remove(key: string): void {
    const found = this.#held.get(key);
    if (found === undefined) return;
    this.#size -= found.bytes.byteLength;
    this.#held.delete(key);
  }
}

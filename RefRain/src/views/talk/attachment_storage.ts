// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Option, Result, Schema } from "effect";

import type { DraftAttachment } from "./attachments";

const databaseName = "refrain-attachment-drafts";
const table = "attachments";
const index = "draft";
const operations = "operations";
const region = Schema.Struct({
  x: Schema.Number,
  y: Schema.Number,
  width: Schema.Number,
  height: Schema.Number,
});
const stored = Schema.Array(
  Schema.Struct({
    id: Schema.String,
    workspace: Schema.String,
    file: Schema.String,
    label: Schema.String,
    version: Schema.String,
    selection: Schema.Union([
      Schema.Struct({
        kind: Schema.Literal("markdown"),
        from: Schema.Int,
        to: Schema.Int,
        text: Schema.String,
      }),
      Schema.Struct({
        kind: Schema.Literal("pdf-text"),
        page: Schema.Int,
        region,
        text: Schema.String,
      }),
      Schema.Struct({
        kind: Schema.Literal("pdf-region"),
        page: Schema.Int,
        region,
        image: Schema.instanceOf(Blob),
      }),
    ]),
  }),
);
const read = Schema.decodeUnknownOption(stored);

let opened: Promise<IDBDatabase | null> | undefined;

function database(): Promise<IDBDatabase | null> {
  opened ??= new Promise((resolve) => {
    if (typeof indexedDB === "undefined") {
      resolve(null);
      return;
    }
    const opening = Result.try(() => indexedDB.open(databaseName, 2));
    if (Result.isFailure(opening)) {
      resolve(null);
      return;
    }
    const request = opening.success;
    request.onupgradeneeded = () => {
      const db = request.result;
      if (!db.objectStoreNames.contains(table))
        db.createObjectStore(table, { keyPath: "id" }).createIndex(index, "draft");
      if (!db.objectStoreNames.contains(operations)) db.createObjectStore(operations);
    };
    request.onsuccess = () => {
      resolve(request.result);
    };
    request.onerror = () => {
      resolve(null);
    };
    request.onblocked = () => {
      resolve(null);
    };
  });
  return opened;
}

export async function readStoredAttachments(
  draft: string,
): Promise<readonly DraftAttachment[] | null> {
  const db = await database();
  if (db === null) return null;
  return new Promise((resolve) => {
    const opened = Result.try(() =>
      db.transaction(table, "readonly").objectStore(table).index(index).getAll(draft),
    );
    if (Result.isFailure(opened)) {
      resolve(null);
      return;
    }
    const request = opened.success;
    request.onsuccess = () => {
      resolve(Option.getOrNull(read(request.result)));
    };
    request.onerror = () => {
      resolve(null);
    };
  });
}

export async function editStoredAttachments(
  change: (store: IDBObjectStore) => void,
): Promise<boolean> {
  return transact(table, change);
}

export async function readStoredOperation(draft: string): Promise<unknown> {
  const db = await database();
  if (db === null) return undefined;
  return new Promise((resolve) => {
    const opened = Result.try(() =>
      db.transaction(operations, "readonly").objectStore(operations).get(draft),
    );
    if (Result.isFailure(opened)) {
      resolve(undefined);
      return;
    }
    const request = opened.success;
    request.onsuccess = () => {
      resolve(request.result);
    };
    request.onerror = () => {
      resolve(undefined);
    };
  });
}

export async function editStoredOperation(draft: string, flight: unknown): Promise<boolean> {
  return transact(operations, (store) => {
    if (flight === null) store.delete(draft);
    else store.put(flight, draft);
  });
}

async function transact(name: string, change: (store: IDBObjectStore) => void): Promise<boolean> {
  const db = await database();
  if (db === null) return false;
  return new Promise((resolve) => {
    const opened = Result.try(() => {
      const transaction = db.transaction(name, "readwrite");
      change(transaction.objectStore(name));
      return transaction;
    });
    if (Result.isFailure(opened)) {
      resolve(false);
      return;
    }
    const transaction = opened.success;
    transaction.oncomplete = () => {
      resolve(true);
    };
    transaction.onerror = () => {
      resolve(false);
    };
    transaction.onabort = () => {
      resolve(false);
    };
  });
}

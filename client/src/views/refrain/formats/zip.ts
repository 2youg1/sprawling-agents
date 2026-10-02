// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A DOCX is a ZIP archive, and a ZIP archive says how large each part
// will be before anything is inflated - and may lie (client/Spec.lean §4-54).
// This reads the archive's own directory, refuses one that promises
// more than a page should hold, and then inflates every part through
// the platform's `DecompressionStream`, stopping a part the moment it
// grows past what the directory promised. Only an archive that kept
// every promise is handed to the renderer, so the renderer's own
// inflating can never outgrow these bounds.

// The most one archive may unpack to. A long report with its pictures
// unpacks to a few MiB; a tab holding a sixteenfold margin over that
// is still a tab, and an archive past it is refused unopened rather
// than drawn in part.
export const UNPACKED_MAX = 64 * 1024 * 1024;

// The most parts one archive may hold: a report has dozens, and a
// directory of a hundred thousand empty parts is a way to spend a tab.
export const PARTS_MAX = 4096;

export type Unpacked =
  | { readonly kind: "unpacked"; readonly parts: ReadonlyMap<string, Uint8Array> }
  | { readonly kind: "refused"; readonly why: "too_large" | "shape" };

interface Part {
  readonly name: string;
  readonly method: number;
  readonly packed: number;
  readonly size: number;
  readonly header: number;
}

const END_SIGNATURE = 0x06054b50;
const ENTRY_SIGNATURE = 0x02014b50;
const LOCAL_SIGNATURE = 0x04034b50;
// The end record is 22 bytes and may be followed by a comment of up to
// 65535 bytes, so it starts within this many bytes of the end.
const END_REACH = 22 + 0xffff;
const STORED = 0;
const DEFLATED = 8;

const SHAPE: Unpacked = { kind: "refused", why: "shape" };

export function unpack(bytes: Uint8Array): Promise<Unpacked> {
  const listed = directoryOf(bytes);
  if (listed === "shape" || listed === "too_large") return Promise.resolve({ kind: "refused", why: listed });
  return inflateAll(bytes, listed).catch(() => SHAPE);
}

// The archive's directory, or why it is not read: a total past the
// bound, or bytes that are not the shape of a ZIP archive this reads
// (ZIP64 and split archives included, which a DOCX does not need).
function directoryOf(bytes: Uint8Array): readonly Part[] | "shape" | "too_large" {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const end = endOf(view);
  if (end === null) return "shape";
  const count = view.getUint16(end + 10, true);
  let at = view.getUint32(end + 16, true);
  if (count > PARTS_MAX) return "too_large";
  const parts: Part[] = [];
  let total = 0;
  for (let index = 0; index < count; index += 1) {
    if (at + 46 > view.byteLength || view.getUint32(at, true) !== ENTRY_SIGNATURE) return "shape";
    const naming = view.getUint16(at + 28, true);
    const part: Part = {
      method: view.getUint16(at + 10, true),
      packed: view.getUint32(at + 20, true),
      size: view.getUint32(at + 24, true),
      header: view.getUint32(at + 42, true),
      name: new TextDecoder().decode(bytes.subarray(at + 46, at + 46 + naming)),
    };
    total += part.size;
    if (total > UNPACKED_MAX) return "too_large";
    parts.push(part);
    at += 46 + naming + view.getUint16(at + 30, true) + view.getUint16(at + 32, true);
  }
  return parts;
}

function endOf(view: DataView): number | null {
  for (let at = view.byteLength - 22; at >= Math.max(0, view.byteLength - END_REACH); at -= 1) {
    if (view.getUint32(at, true) === END_SIGNATURE) return at;
  }
  return null;
}

async function inflateAll(bytes: Uint8Array, parts: readonly Part[]): Promise<Unpacked> {
  const out = new Map<string, Uint8Array>();
  for (const part of parts) {
    const data = dataOf(bytes, part);
    const unpacked = data === null ? null : await inflate(data, part);
    if (unpacked === null) return SHAPE;
    out.set(part.name, unpacked);
  }
  return { kind: "unpacked", parts: out };
}

// The packed bytes of one part, found through its local header.
function dataOf(bytes: Uint8Array, part: Part): Uint8Array | null {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const at = part.header;
  if (at + 30 > view.byteLength || view.getUint32(at, true) !== LOCAL_SIGNATURE) return null;
  const start = at + 30 + view.getUint16(at + 26, true) + view.getUint16(at + 28, true);
  return start + part.packed > view.byteLength ? null : bytes.subarray(start, start + part.packed);
}

// One part, exactly as large as the directory said, or `null` when it
// is stored in a way this does not read or does not keep that promise.
async function inflate(data: Uint8Array, part: Part): Promise<Uint8Array | null> {
  if (part.method === STORED) return data.length === part.size ? data : null;
  if (part.method !== DEFLATED) return null;
  const reader = new Blob([data.slice()]).stream().pipeThrough(new DecompressionStream("deflate-raw")).getReader();
  const out = new Uint8Array(part.size);
  let filled = 0;
  for (;;) {
    const read = await reader.read();
    if (read.done) return filled === part.size ? out : null;
    if (filled + read.value.length > part.size) {
      void reader.cancel();
      return null;
    }
    out.set(read.value, filled);
    filled += read.value.length;
  }
}

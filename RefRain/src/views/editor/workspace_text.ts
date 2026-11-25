// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Result } from "effect";
import { positionsOf, textEdits } from "../../core/document_pos";
import type { EditorChange, Positions } from "../../core/document_pos";
import { EDITABLE_BYTES_MAX } from "../../core/document_windows";
import type { Encoding } from "../../wire";
import type { DocumentRewrite } from "../talk/attachments";

const DECODER: Readonly<Record<Encoding, string>> = {
  utf8: "utf-8",
  utf8_bom: "utf-8",
  utf16_le: "utf-16le",
  utf16_be: "utf-16be",
};

function encodingOf(bytes: Uint8Array): Encoding {
  if (bytes[0] === 0xef && bytes[1] === 0xbb && bytes[2] === 0xbf) return "utf8_bom";
  if (bytes[0] === 0xff && bytes[1] === 0xfe) return "utf16_le";
  if (bytes[0] === 0xfe && bytes[1] === 0xff) return "utf16_be";
  return "utf8";
}

export function decodeWorkspaceText(version: string, bytes: Uint8Array): Positions<string> | null {
  const encoding = encodingOf(bytes);
  if (encoding === "utf8" && bytes.includes(0)) return null;
  const decoded = Result.try(() =>
    new TextDecoder(DECODER[encoding], { fatal: true, ignoreBOM: true }).decode(bytes),
  );
  return Result.isSuccess(decoded) ? positionsOf(version, encoding, decoded.success) : null;
}

export function workspaceExcerpt(version: string, bytes: Uint8Array): Positions<string> | null {
  if (bytes.length <= EDITABLE_BYTES_MAX) return decodeWorkspaceText(version, bytes);
  for (let back = 0; back < 4; back += 1) {
    const decoded = decodeWorkspaceText(version, bytes.subarray(0, EDITABLE_BYTES_MAX - back));
    if (decoded !== null) return decoded;
  }
  return null;
}

function encode(text: string, encoding: Encoding): Uint8Array<ArrayBuffer> {
  if (encoding === "utf8" || encoding === "utf8_bom") return new TextEncoder().encode(text);
  const encoded = new Uint8Array(text.length * 2);
  const into = new DataView(encoded.buffer);
  for (let at = 0; at < text.length; at++)
    into.setUint16(at * 2, text.charCodeAt(at), encoding === "utf16_le");
  return encoded;
}

export function workspaceLineBytes(line: string, encoding: Encoding): number {
  return encoding === "utf16_le" || encoding === "utf16_be"
    ? line.length * 2
    : new TextEncoder().encode(line).byteLength;
}

export function workspaceDraftBytes(
  savedBytes: number,
  baseline: Positions<string>,
  changes: readonly EditorChange[],
): number {
  return textEdits(baseline, changes).reduce(
    (total, { span, text }) =>
      total + workspaceLineBytes(text, baseline.encoding) - (span.end - span.start),
    savedBytes,
  );
}

export function encodeWorkspaceChanges(
  original: Uint8Array,
  baseline: Positions<string>,
  changes: readonly EditorChange[],
): Uint8Array<ArrayBuffer> {
  const parts: Uint8Array[] = [];
  let from = 0;
  for (const { span, text } of textEdits(baseline, changes)) {
    parts.push(original.subarray(from, span.start), encode(text, baseline.encoding));
    from = span.end;
  }
  parts.push(original.subarray(from));
  const output = new Uint8Array(parts.reduce((length, part) => length + part.length, 0));
  let written = 0;
  for (const part of parts) {
    output.set(part, written);
    written += part.length;
  }
  return output;
}

export type ProposalBlock = "version" | "overlap" | "invalid" | "readonly";

export function proposalBlock(
  proposal: DocumentRewrite,
  baseline: Positions<string> | null,
  changes: readonly EditorChange[],
): ProposalBlock | null {
  if (proposal.version !== baseline?.version) return "version";
  const { from, to, quote } = proposal;
  if (
    !Number.isSafeInteger(from) ||
    !Number.isSafeInteger(to) ||
    from < 0 ||
    from >= to ||
    to > baseline.editor.length ||
    quote !== baseline.editor.slice(from, to) ||
    baseline.editorAt(baseline.bytes(from)) !== from ||
    baseline.editorAt(baseline.bytes(to)) !== to
  )
    return "invalid";
  if (changes.some((change) => change.from <= to && change.to >= from)) return "overlap";
  return null;
}

export function validReplacement(text: string): boolean {
  return !/[\ud800-\udfff]/u.test(text);
}

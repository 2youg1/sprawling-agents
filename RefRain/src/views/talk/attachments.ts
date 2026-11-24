// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { derived, writable } from "../../core/store";
import { editStoredAttachments, readStoredAttachments } from "./attachment_storage";

export interface PdfRegion {
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
}

export type DocumentSelection =
  | {
      readonly kind: "pdf-text";
      readonly page: number;
      readonly region: PdfRegion;
      readonly text: string;
    }
  | {
      readonly kind: "pdf-region";
      readonly page: number;
      readonly region: PdfRegion;
      readonly image: Blob;
    }
  | {
      readonly kind: "markdown";
      readonly from: number;
      readonly to: number;
      readonly text: string;
    };

export interface DocumentAttachment {
  readonly workspace: string;
  readonly file: string;
  readonly version: string;
  readonly label: string;
  readonly selection: DocumentSelection;
}

export interface DraftAttachment extends DocumentAttachment {
  readonly id: string;
}

export type DocumentCitation = Pick<
  DocumentAttachment,
  "workspace" | "file" | "version" | "label"
> & {
  readonly selection:
    | Exclude<DocumentSelection, { readonly kind: "pdf-region" }>
    | Omit<Extract<DocumentSelection, { readonly kind: "pdf-region" }>, "image">;
};

export interface DocumentRewrite extends Pick<
  DocumentAttachment,
  "workspace" | "file" | "version" | "label"
> {
  readonly from: number;
  readonly to: number;
  readonly quote: string;
  readonly replacement: string;
}

const empty: readonly DraftAttachment[] = [];
const drafts = writable<ReadonlyMap<string, readonly DraftAttachment[]>>(new Map());
const storageFailures = writable<ReadonlySet<string>>(new Set());
const loaded = new Map<string, Promise<void>>();
const request = writable<DocumentCitation | null>(null);
const proposal = writable<DocumentRewrite | null>(null);

export const requestedCitation = {
  subscribe: (take: (value: DocumentCitation | null) => void) => request.subscribe(take),
};
export const requestedProposal = {
  subscribe: (take: (value: DocumentRewrite | null) => void) => proposal.subscribe(take),
};

export function openCitation(citation: DocumentCitation): void {
  request.set(citation);
}

export function reviewRewrite(rewrite: DocumentRewrite): void {
  proposal.set(rewrite);
}

export function attachmentsAt(key: string | undefined) {
  return derived(drafts, ($drafts) => (key === undefined ? empty : ($drafts.get(key) ?? empty)));
}

export function attachmentStorageAt(key: string | undefined) {
  return derived(storageFailures, ($failed) => key !== undefined && $failed.has(key));
}

function storageFailed(key: string): void {
  storageFailures.update((failed) => new Set(failed).add(key));
}

export function loadAttachments(key: string): Promise<void> {
  const underway = loaded.get(key);
  if (underway !== undefined) return underway;
  const reading = readStoredAttachments(key).then((saved) => {
    if (saved === null) {
      storageFailed(key);
      return;
    }
    if (saved.length === 0) return;
    drafts.update((held) => new Map(held).set(key, [...saved, ...(held.get(key) ?? empty)]));
  });
  loaded.set(key, reading);
  return reading;
}

export function keepAttachment(key: string, attachment: DocumentAttachment): void {
  const item = { ...attachment, id: crypto.randomUUID() };
  drafts.update((held) => new Map(held).set(key, [...(held.get(key) ?? empty), item]));
  void loadAttachments(key).then(async () => {
    if (
      !(await editStoredAttachments((store) => {
        store.put({ ...item, draft: key });
      }))
    )
      storageFailed(key);
  });
}

export function removeAttachments(key: string, ids: readonly string[]): void {
  if (ids.length === 0) return;
  drafts.update((held) => {
    const rest = (held.get(key) ?? empty).filter((part) => !ids.includes(part.id));
    const next = new Map(held);
    if (rest.length === 0) next.delete(key);
    else next.set(key, rest);
    return next;
  });
  void loadAttachments(key).then(async () => {
    if (
      !(await editStoredAttachments((store) => {
        for (const id of ids) store.delete(id);
      }))
    )
      storageFailed(key);
  });
}

export function citationOf({
  workspace,
  file,
  version,
  label,
  selection,
}: DocumentAttachment): DocumentCitation {
  return {
    workspace,
    file,
    version,
    label,
    selection:
      selection.kind === "pdf-region"
        ? { kind: selection.kind, page: selection.page, region: selection.region }
        : selection,
  };
}

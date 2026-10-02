import type { Address, AxError, B3Hash, Command, EventRecord } from "../wire";
import { putRange } from "./commands/document";
import type { EditorChange, Positions } from "./document_pos";
export interface Draft { readonly version: B3Hash; readonly changes: readonly EditorChange[] }
export function draftPlace(doc: Address): string { return doc; }
export function writeDraft(_d: Draft): string { return ""; }
export function readDraft(_s: string, _l: number): Draft | null { return null; }
export interface Sent { readonly doc: Address; readonly baseline: B3Hash; readonly command: Command }
export function saveOf(doc: Address, positions: Positions, _c: readonly EditorChange[]): Sent { return { doc, baseline: positions.version, command: putRange(doc, positions.version, []) }; }
export function receiptIn(_r: readonly EventRecord[], _s: Sent): B3Hash | null { return null; }
export type Receipt = { readonly kind: "clean" } | { readonly kind: "draft" } | { readonly kind: "saving"; readonly sent: Sent; readonly edited: boolean } | { readonly kind: "pending"; readonly sent: Sent; readonly edited: boolean } | { readonly kind: "saved"; readonly version: B3Hash } | { readonly kind: "conflict"; readonly current: B3Hash | null } | { readonly kind: "refused"; readonly error: AxError };
export type Happened = { readonly kind: "edited"; readonly empty: boolean } | { readonly kind: "based"; readonly empty: boolean } | { readonly kind: "sent"; readonly sent: Sent } | { readonly kind: "lost" } | { readonly kind: "relinked" } | { readonly kind: "landed"; readonly version: B3Hash } | { readonly kind: "refusal"; readonly error: AxError } | { readonly kind: "moved"; readonly version: B3Hash };
export function advance(r: Receipt, _h: Happened): Receipt { return r; }

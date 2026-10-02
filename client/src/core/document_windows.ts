import type { B3Hash, DocumentAnswer, Encoding, Format, RangeAnswer, Span } from "../wire";
export const EDITABLE_BYTES_MAX = 4 * 1024 * 1024;
export interface Gathering { readonly version: B3Hash; readonly format: Format; readonly encoding: Encoding; readonly bytes: number; readonly text: string; readonly through: number }
export type Opened = { readonly kind: "missing" } | { readonly kind: "unreadable"; readonly reason: string } | { readonly kind: "opaque"; readonly version: B3Hash; readonly bytes: number } | { readonly kind: "text"; readonly gathering: Gathering };
export function opened(_answer: DocumentAnswer): Opened { return { kind: "missing" }; }
export function nextSpan(_g: Gathering): Span | null { return null; }
export function joined(g: Gathering, _a: RangeAnswer): Gathering { return g; }
export function whole(_g: Gathering): boolean { return true; }

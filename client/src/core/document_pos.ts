import type { B3Hash, Encoding, TextEdit } from "../wire";
export interface EditorChange { readonly from: number; readonly to: number; readonly insert: string }
export interface Place { readonly line: number; readonly column: number }
export interface Positions {
  readonly version: B3Hash; readonly encoding: Encoding; readonly editor: string; readonly lineBreak: string;
  readonly bytes: (at: number) => number; readonly editorAt: (byte: number) => number; readonly place: (at: number) => Place;
}
export function positionsOf(version: B3Hash, encoding: Encoding, text: string): Positions {
  return { version, encoding, editor: text, lineBreak: "\n", bytes: (at) => at, editorAt: (at) => at, place: () => ({ line: 1, column: 1 }) };
}
export function textEdits(_positions: Positions, changes: readonly EditorChange[]): TextEdit[] {
  return changes.map((each) => ({ span: { start: each.from, end: each.to }, text: each.insert }));
}

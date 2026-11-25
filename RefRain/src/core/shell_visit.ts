// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Option, Schema } from "effect";

import { DEFAULT_VIEW, fromFragment, toFragment, type View, type WorkspaceView } from "./route";
import { browserRows, type Rows } from "./rows";
import type { WorkspaceProgress } from "./workspace";

export type Subject = "files" | "session";
export type NavigationLayer = "list" | "history" | "tools" | "library";
export type Navigation = "closed" | NavigationLayer;

export const DEFAULT_SPLIT_LINE = 8;
export const START_FILE_LINE = 2;
export const LAST_SPLIT_LINE = 11;
export const DEFAULT_SPLIT_TOP = 50;
export const MIN_SPLIT_TOP = 25;
export const MAX_SPLIT_TOP = 75;

const KEYS = {
  body: "refrain.workspace.subject.v1",
  talk: "refrain.workspace.talk.v1",
  split: "refrain.workspace.split.v1",
  splitLine: "refrain.workspace.split-line.v1",
  splitTop: "refrain.workspace.split-top.v1",
  workspace: "refrain.workspace.active.v1",
  file: "refrain.workspace.file.v1.",
  pdfProgress: "refrain.workspace.pdf-progress.v1.",
};

const readProgress = Schema.decodeUnknownOption(
  Schema.fromJsonString(
    Schema.Struct({
      page: Schema.Int.check(Schema.isGreaterThanOrEqualTo(1)),
      offset: Schema.Number.check(Schema.isBetween({ minimum: 0, maximum: 1 })),
    }),
  ),
);

export function shellVisit(rows: Rows = browserRows()) {
  return {
    subject(): Subject {
      return rows.getItem(KEYS.body) === "files" ? "files" : "session";
    },
    setSubject(subject: Subject): void {
      rows.setItem(KEYS.body, subject);
    },
    talk(): View {
      const saved = Option.getOrNull(fromFragment(rows.getItem(KEYS.talk) ?? ""));
      return saved?.kind === "talk" ? saved : DEFAULT_VIEW;
    },
    setTalk({ address, session }: WorkspaceView): void {
      const target: WorkspaceView =
        session === undefined ? { kind: "talk", address } : { kind: "talk", address, session };
      rows.setItem(KEYS.talk, toFragment(target));
    },
    split(): boolean {
      return rows.getItem(KEYS.split) === "yes";
    },
    setSplit(split: boolean): void {
      rows.setItem(KEYS.split, split ? "yes" : "no");
    },
    splitLine(): number {
      const line = Number(rows.getItem(KEYS.splitLine));
      return Number.isInteger(line) && line > START_FILE_LINE + 1 && line <= LAST_SPLIT_LINE
        ? line
        : DEFAULT_SPLIT_LINE;
    },
    setSplitLine(line: number): void {
      rows.setItem(KEYS.splitLine, String(line));
    },
    splitTop(): number {
      const part = Number(rows.getItem(KEYS.splitTop));
      return Number.isInteger(part) && part >= MIN_SPLIT_TOP && part <= MAX_SPLIT_TOP
        ? part
        : DEFAULT_SPLIT_TOP;
    },
    setSplitTop(part: number): void {
      rows.setItem(KEYS.splitTop, String(part));
    },
    workspace(): string | null {
      return rows.getItem(KEYS.workspace);
    },
    setWorkspace(id: string): void {
      rows.setItem(KEYS.workspace, id);
    },
    file(workspace: string): string | null {
      return rows.getItem(KEYS.file + workspace);
    },
    pdfProgress(key: string): WorkspaceProgress | undefined {
      return Option.getOrUndefined(readProgress(rows.getItem(KEYS.pdfProgress + key)));
    },
    setPdfProgress(key: string, point: WorkspaceProgress): void {
      rows.setItem(KEYS.pdfProgress + key, JSON.stringify(point));
    },
    clearPdfProgress(key: string): void {
      rows.removeItem(KEYS.pdfProgress + key);
    },
    closeFile(workspace: string): void {
      rows.removeItem(KEYS.file + workspace);
    },
    setFile(workspace: string, file: string): void {
      rows.setItem(KEYS.file + workspace, file);
    },
  };
}

export type ShellVisit = ReturnType<typeof shellVisit>;

export function launchView(hash: string, visit: ShellVisit): View {
  if (hash === "") return visit.talk();
  return Option.getOrElse(fromFragment(hash), () => DEFAULT_VIEW);
}

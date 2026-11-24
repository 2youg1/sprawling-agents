// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Effect, Option, Schema } from "effect";

import { AxError } from "../wire";
import { fill, langOf, say, type Key } from "./lang";
import { bearing } from "./socket";

const ReadingPosition = Schema.Struct({ page: Schema.Int, offset: Schema.Number });
const Record = Schema.Struct({
  id: Schema.String,
  path: Schema.String,
  version: Schema.String,
  progress: Schema.NullOr(ReadingPosition),
  category: Schema.NullOr(Schema.String),
  lastOpened: Schema.Number,
});
const Workspace = Schema.Struct({
  id: Schema.String,
  name: Schema.String,
  root: Schema.String,
  recent: Schema.Array(Record),
});
const Entry = Schema.Struct({
  name: Schema.String,
  path: Schema.String,
  kind: Schema.Literals(["directory", "file"]),
});
const Revision = Schema.Struct({ revision: Schema.optional(Schema.NullOr(Schema.String)) });
const Page = Schema.Struct({
  entries: Schema.Array(Entry),
  next: Schema.NullOr(Schema.String),
  ...Revision.fields,
});
const Workspaces = Schema.Array(Workspace);

export type WorkspaceProgress = typeof ReadingPosition.Type;
export type WorkspaceRecord = typeof Record.Type;
export type WorkspaceInfo = typeof Workspace.Type;
export type WorkspaceEntry = typeof Entry.Type;
export type WorkspacePage = typeof Page.Type;

export interface WorkspaceChange {
  readonly progress?: WorkspaceProgress;
  readonly category?: string | null;
}

export interface WorkspaceClient {
  readonly workspaces: () => Promise<readonly WorkspaceInfo[]>;
  readonly register: (root: string) => Promise<WorkspaceInfo>;
  readonly list: (workspace: string, path?: string, cursor?: string) => Promise<WorkspacePage>;
  readonly observe: (
    workspace: string,
    path: string,
    revision: string,
    signal: AbortSignal,
  ) => Promise<typeof Revision.Type>;
  readonly open: (
    workspace: string,
    target: { path: string } | { id: string },
  ) => Promise<WorkspaceRecord>;
  readonly bytes: (
    workspace: string,
    id: string,
    version: string,
  ) => Promise<Uint8Array<ArrayBuffer>>;
  readonly resource: (workspace: string, path: string) => Promise<Uint8Array<ArrayBuffer>>;
  readonly remember: (
    workspace: string,
    id: string,
    change: WorkspaceChange,
  ) => Promise<WorkspaceRecord>;
  readonly relink: (workspace: string, id: string, path: string) => Promise<WorkspaceRecord>;
  readonly save: (
    workspace: string,
    id: string,
    baseline: string,
    bytes: Uint8Array,
  ) => Promise<WorkspaceRecord>;
}

export class WorkspaceError extends Error {
  constructor(readonly diagnostic: AxError) {
    super(`${diagnostic.action}: ${diagnostic.subject}. ${diagnostic.recovery}`);
    this.name = "WorkspaceError";
  }
}

export function createWorkspace(origin: string, credential: () => string | null): WorkspaceClient {
  function post<A>(body: { kind: string }, decode: Parse<A>): Promise<A> {
    return Effect.runPromise(
      checked(body.kind, `${origin}/workspace`, {
        method: "POST",
        headers: { "content-type": "application/json", ...bearing(credential()) },
        body: JSON.stringify(body),
        ...(body.kind === "remember" ? { keepalive: true } : {}),
      }).pipe(Effect.flatMap((answer) => decodeReply(answer, decode, body.kind))),
    );
  }
  const request = <A>(kind: string, fields: object, schema: Parse<A>) =>
    post({ kind, ...fields }, schema);
  const workspaces = Schema.decodeUnknownOption(Workspaces);
  const workspace = Schema.decodeUnknownOption(Workspace);
  const page = Schema.decodeUnknownOption(Page);
  const record = Schema.decodeUnknownOption(Record);
  const revisionReply = Schema.decodeUnknownOption(Revision);

  function binary(kind: string, fields: object): Promise<Uint8Array<ArrayBuffer>> {
    const action = "read workspace bytes";
    return Effect.runPromise(
      checked(action, `${origin}/workspace`, {
        method: "POST",
        headers: { "content-type": "application/json", ...bearing(credential()) },
        body: JSON.stringify({ kind, ...fields }),
      }).pipe(
        Effect.flatMap((answer) => {
          if (answer.headers.get("content-type") !== "application/octet-stream") {
            return Effect.fail(unavailable(action, phrase("workspace_response_not_bytes")));
          }
          return Effect.tryPromise({
            try: async () => new Uint8Array(await answer.arrayBuffer()),
            catch: () => unavailable(action, phrase("workspace_response_not_bytes")),
          });
        }),
      ),
    );
  }

  return {
    workspaces: () => request("workspaces", {}, workspaces),
    register: (root) => request("register", { root }, workspace),
    list: (workspace, path = "", cursor) => request("list", { workspace, path, cursor }, page),
    observe(workspace, path, revision, signal) {
      const action = "observe workspace directory";
      return Effect.runPromise(
        checked(action, `${origin}/workspace`, {
          method: "POST",
          headers: { "content-type": "application/json", ...bearing(credential()) },
          body: JSON.stringify({ kind: "watch", workspace, path, revision }),
          signal,
        }).pipe(Effect.flatMap((answer) => decodeReply(answer, revisionReply, action))),
      );
    },
    open: (workspace, target) => request("open", { workspace, target }, record),
    bytes: (workspace, id, version) => binary("bytes", { workspace, id, version }),
    resource: (workspace, path) => binary("resource", { workspace, path }),
    remember(workspace, id, { progress, category }) {
      const changes = [];
      if (progress !== undefined) changes.push({ kind: "progress", progress });
      if (category !== undefined) changes.push({ kind: "category", category });
      return request("remember", { workspace, id, changes }, record);
    },
    relink: (workspace, id, path) => request("relink", { workspace, id, path }, record),
    save(workspace, id, baseline, bytes) {
      const action = "save workspace file";
      const query = new URLSearchParams({ workspace, id, baseline });
      return Effect.runPromise(
        checked(action, `${origin}/workspace/save?${query}`, {
          method: "POST",
          headers: { "content-type": "application/octet-stream", ...bearing(credential()) },
          body: new Uint8Array(bytes),
        }).pipe(Effect.flatMap((answer) => decodeReply(answer, record, action))),
      );
    },
  };
}

type Parse<A> = (value: unknown) => Option.Option<A>;

const errorOf = Schema.decodeUnknownOption(AxError);

const checked = Effect.fnUntraced(function* (action: string, url: string, init: RequestInit) {
  const answer = yield* Effect.tryPromise({
    try: () => fetch(url, init),
    catch: () => unavailable(action, phrase("workspace_no_response")),
  });
  if (answer.ok) return answer;
  const value = yield* Effect.option(jsonReply(answer, action));
  const error = Option.getOrNull(Option.flatMap(value, errorOf));
  return yield* Effect.fail(
    error === null
      ? unavailable(
          action,
          fill(phrase("workspace_unreadable_http"), { status: String(answer.status) }),
        )
      : new WorkspaceError(error),
  );
});

function jsonReply(answer: Response, action: string): Effect.Effect<unknown, WorkspaceError> {
  return Effect.tryPromise({
    try: () => answer.json(),
    catch: () => unavailable(action, phrase("workspace_unreadable_response")),
  });
}

function decodeReply<A>(
  answer: Response,
  decode: Parse<A>,
  action: string,
): Effect.Effect<A, WorkspaceError> {
  return jsonReply(answer, action).pipe(
    Effect.flatMap((value) =>
      Option.match(decode(value), {
        onSome: Effect.succeed,
        onNone: () => Effect.fail(unavailable(action, phrase("workspace_unreadable_response"))),
      }),
    ),
  );
}

function unavailable(action: string, subject: string): WorkspaceError {
  return new WorkspaceError({
    action,
    code: "E_TOOL_UNAVAILABLE",
    subject,
    nearby: [],
    retry: "unknown",
    recovery: phrase("workspace_reconnect"),
  });
}

function phrase(key: Key): string {
  return say(langOf(typeof document === "undefined" ? "en" : document.documentElement.lang), key);
}

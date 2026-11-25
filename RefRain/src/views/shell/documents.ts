// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Effect, Result } from "effect";
import { createEffect, createMemo, createSignal, onSettled } from "solid-js";

import { toFragment, type ItemLink } from "../../core/route";
import { get } from "../../core/store";
import {
  createWorkspace,
  WorkspaceError,
  type WorkspaceInfo,
  type WorkspaceProgress,
  type WorkspaceRecord,
} from "../../core/workspace";
import { ui } from "../../ui";
import type { Address, Seq } from "../../wire";
import {
  keepAttachment,
  requestedCitation,
  requestedProposal,
  type DocumentAttachment,
  type DocumentCitation,
  type DocumentRewrite,
  type DocumentSelection,
} from "../talk/attachments";
import { draftPlace } from "../talk/draft";
import { cacheKey, FileCache, type OpenedFile } from "./file_cache";
import type { LibraryTarget } from "./file_library";
import { focusEdgeKey, NAVIGATION_KEY } from "./chrome";
import { focusComposer } from "./focus";
import type { ShellLayout } from "./layout";
import { storeValue } from "../../core/solid_store";

export interface DocumentTarget {
  readonly address: Address;
  readonly session?: Seq | undefined;
  readonly item?: ItemLink | undefined;
  readonly shell: ShellLayout;
}

export function workspaceDocuments(target: DocumentTarget) {
  const u = ui();
  const api = createWorkspace(u.origin, u.credential);
  const cache = new FileCache();
  const branches = storeValue(u.branches.held);
  const askedCitation = storeValue(requestedCitation);
  const askedProposal = storeValue(requestedProposal);

  const [spaces, setSpaces] = createSignal<readonly WorkspaceInfo[]>([]);
  const [workspace, setWorkspace] = createSignal<string | null>(null);
  const [opened, setOpened] = createSignal<OpenedFile | null>(null);
  const [failed, setFailed] = createSignal<unknown>(null);
  const [selection, setSelection] = createSignal<DocumentAttachment | null>(null);
  const [review, setReview] = createSignal<DocumentRewrite | null>(null);
  const [citation, setCitation] = createSignal<DocumentCitation | null>(null);
  const [pending, setPending] = createSignal(false);
  const [remembered, setRemembered] = createSignal<WorkspaceProgress>();
  const [recovery, setRecovery] = createSignal<{
    readonly workspace: string;
    readonly id: string;
  } | null>(null);
  const [repair, setRepair] = createSignal("");
  const [choosing, setChoosing] = createSignal(false);
  let libraryLoaded = false;
  let load = 0;
  let focusBack: HTMLElement | null = null;
  let visible = true;

  const reviewing = createMemo(() => {
    const asked = review();
    const file = opened();
    return asked !== null && file?.workspace === asked.workspace && file.record.id === asked.file
      ? asked
      : undefined;
  });
  const changed = createMemo(() => {
    const reference = citation();
    const file = opened();
    return (
      reference !== null &&
      file?.workspace === reference.workspace &&
      file.record.id === reference.file &&
      file.record.version !== reference.version
    );
  });
  const revealed = createMemo(() => {
    const source = citation();
    const file = opened();
    return source !== null &&
      file?.workspace === source.workspace &&
      file.record.id === source.file &&
      file.record.version === source.version
      ? source.selection
      : undefined;
  });

  function touch(workspaceId: string, record: WorkspaceRecord): void {
    setSpaces((held) =>
      held.map((space) =>
        space.id !== workspaceId
          ? space
          : {
              ...space,
              recent: [record, ...space.recent.filter((each) => each.id !== record.id)].slice(
                0,
                20,
              ),
            },
      ),
    );
  }

  const recording = new Set<string>();

  async function rememberProgress(file: OpenedFile, key: string): Promise<void> {
    if (recording.has(key)) return;
    const visit = target.shell.visit;
    recording.add(key);
    let point = visit.pdfProgress(key);
    while (point !== undefined) {
      const requested = point;
      const result = await Effect.runPromise(
        Effect.result(
          Effect.tryPromise({
            try: () => api.remember(file.workspace, file.record.id, { progress: requested }),
            catch: (error) => error,
          }),
        ),
      );
      point = visit.pdfProgress(key);
      const samePoint = point?.page === requested.page && point.offset === requested.offset;
      if (Result.isFailure(result)) {
        if (!samePoint) continue;
        const current = opened();
        if (
          visible &&
          current?.workspace === file.workspace &&
          current.record.id === file.record.id
        )
          setFailed(result.failure);
        break;
      }
      const record = result.success;
      if (visible) touch(file.workspace, record);
      if (samePoint) {
        visit.clearPdfProgress(key);
        break;
      }
    }
    recording.delete(key);
  }

  function progress(file: OpenedFile, point: WorkspaceProgress): void {
    const key = cacheKey(file.workspace, file.record.id);
    target.shell.visit.setPdfProgress(key, point);
    void rememberProgress(file, key);
  }

  function display(file: OpenedFile, queued?: WorkspaceProgress): void {
    const { workspace, record } = file;
    target.shell.visit.setFile(workspace, record.id);
    const previous = opened();
    if (previous?.workspace !== workspace || previous.record.id !== record.id) {
      const key = cacheKey(workspace, record.id);
      const pending = target.shell.visit.pdfProgress(key);
      setRemembered(pending ?? queued ?? record.progress ?? undefined);
      if (pending !== undefined) void rememberProgress(file, key);
    }
    setOpened(file);
    touch(workspace, record);
  }

  function open(id: string, file: LibraryTarget, from?: DocumentCitation): void {
    const request = ++load;
    setFailed(null);
    setPending(true);
    setCitation(from ?? null);
    setRecovery("id" in file ? { workspace: id, id: file.id } : null);
    setRepair("");
    const cached = "id" in file ? cache.get(id, file.id) : undefined;
    const queued = "id" in file ? target.shell.visit.pdfProgress(cacheKey(id, file.id)) : undefined;
    if (
      cached !== undefined &&
      !cached.record.path.toLowerCase().endsWith(".pdf") &&
      (from === undefined || cached.record.version === from.version)
    )
      display(cached);
    void api
      .open(id, file)
      .then(async (record) => {
        if (request !== load) return;
        const previous = cache.get(id, record.id);
        const next = {
          workspace: id,
          record,
          bytes:
            previous?.record.version === record.version
              ? previous.bytes
              : await api.bytes(id, record.id, record.version),
        };
        if (request !== load) return;
        cache.keep(next);
        setPending(false);
        display(next, queued);
        setRecovery(null);
      })
      .catch((error: unknown) => {
        if (request !== load) return;
        setPending(false);
        if (cached !== undefined) {
          setOpened(null);
          cache.delete(cached.workspace, cached.record.id);
        }
        setFailed(error);
      });
  }

  function activate(id: string, next?: { id: string }): void {
    if (workspace() !== id) setOpened(null);
    setWorkspace(id);
    target.shell.visit.setWorkspace(id);
    const previous = next?.id ?? target.shell.visit.file(id);
    if (previous !== null) open(id, { id: previous });
    else {
      ++load;
      setPending(false);
    }
  }

  function closeFile(): void {
    ++load;
    const file = opened();
    if (file !== null) {
      target.shell.visit.closeFile(file.workspace);
      cache.delete(file.workspace, file.record.id);
    }
    setPending(false);
    setOpened(null);
    setReview(null);
    setCitation(null);
    setFailed(null);
    focusEdgeKey(NAVIGATION_KEY);
  }

  function refreshLibrary(): void {
    void api.workspaces().then(
      (records) => {
        if (!visible) return;
        setSpaces(records);
        const wanted = target.shell.visit.workspace() ?? records[0]?.id ?? null;
        if (workspace() === null && wanted !== null) activate(wanted);
      },
      (error: unknown) => {
        if (!visible) return;
        setFailed(error);
        libraryLoaded = false;
      },
    );
  }

  function register(space: WorkspaceInfo): void {
    setSpaces((held) => [space, ...held.filter((each) => each.id !== space.id)]);
    activate(space.id);
  }

  async function save(
    file: OpenedFile,
    baseline: string,
    bytes: Uint8Array,
  ): Promise<WorkspaceRecord> {
    const record = await api.save(file.workspace, file.record.id, baseline, bytes);
    cache.delete(file.workspace, record.id);
    touch(file.workspace, record);
    setOpened((held) =>
      held?.workspace === file.workspace && held.record.id === record.id
        ? { ...held, record, bytes: new Uint8Array(bytes) }
        : held,
    );
    return record;
  }

  function category(file: OpenedFile, category: string | null): void {
    void api.remember(file.workspace, file.record.id, { category }).then(
      (record) => {
        touch(file.workspace, record);
        setOpened((held) =>
          held?.workspace === file.workspace && held.record.id === record.id
            ? { ...held, record: { ...held.record, category: record.category } }
            : held,
        );
      },
      (error: unknown) => {
        setFailed(error);
      },
    );
  }

  function relink(event: SubmitEvent): void {
    event.preventDefault();
    const stored = recovery();
    const path = repair().trim();
    if (stored === null || path === "") return;
    void api.relink(stored.workspace, stored.id, path).then(
      () => {
        open(stored.workspace, { id: stored.id });
      },
      (error: unknown) => {
        setFailed(error);
      },
    );
  }

  function finish(attachment: DocumentAttachment, room: Address, session: Seq | undefined): void {
    keepAttachment(
      draftPlace(room, session ?? get(u.conn.belief).sessions[room] ?? null),
      attachment,
    );
    setSelection(null);
    if (!target.shell.draft && !target.shell.split) target.shell.showDraft();
    onSettled(() => {
      focusComposer();
    });
  }

  async function readResource(
    file: OpenedFile,
    relative: string,
  ): Promise<{ bytes: Uint8Array; name: string }> {
    const parts = file.record.path.split("/").slice(0, -1);
    for (const segment of relative.split("/")) {
      if (segment === "" || segment === ".") continue;
      if (segment === "..") {
        if (parts.pop() !== undefined) continue;
        return Promise.reject(new Error(relative));
      }
      parts.push(segment);
    }
    const name = parts.join("/");
    return { name, bytes: await api.resource(file.workspace, name) };
  }

  function attach(file: OpenedFile, range: DocumentSelection, version: string): void {
    const attachment = {
      workspace: file.workspace,
      file: file.record.id,
      version,
      label: file.record.path,
      selection: range,
    };
    if (target.shell.draft || target.shell.split)
      finish(attachment, target.address, target.session);
    else {
      focusBack = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      setSelection(attachment);
    }
  }

  function useTarget(at: Seq | undefined): void {
    const held = selection();
    if (held === null) return;
    if (at !== undefined && at !== target.session)
      u.go({ kind: "talk", address: target.address, session: at });
    finish(held, target.address, at ?? target.session);
  }

  function newSession(): void {
    const attachment = selection();
    const room = target.address;
    if (attachment === null || choosing()) return;
    setChoosing(
      u.branches.open(room, { kind: "fresh", carry: "nothing" }, () => {
        setChoosing(false);
        finish(attachment, room, undefined);
      }),
    );
  }

  function cancelTarget(): void {
    setSelection(null);
    if (focusBack?.isConnected === true) focusBack.focus({ preventScroll: true });
  }

  function chooseFile(file: LibraryTarget): void {
    const id = workspace();
    if (id === null) return;
    open(id, file);
    target.shell.closeNavigation();
  }

  function link(file: OpenedFile): string {
    return toFragment({
      kind: "talk",
      address: target.address,
      ...(target.session === undefined ? {} : { session: target.session }),
      item: { kind: "workspace-file", workspace: file.workspace, file: file.record.id },
    });
  }

  createEffect(branches, (branch) => {
    if (choosing() && branch.kind === "refused") {
      setChoosing(false);
      setFailed(new WorkspaceError(branch.error));
    }
  });

  createEffect(
    () => target.shell.subject,
    (subject) => {
      if (subject === "files" && !libraryLoaded) {
        libraryLoaded = true;
        refreshLibrary();
      }
    },
  );

  createEffect(
    () => target.item,
    (item) => {
      if (item?.kind === "workspace-file") activate(item.workspace, { id: item.file });
    },
  );

  createEffect(askedCitation, (citation) => {
    if (citation === null) return;
    setWorkspace(citation.workspace);
    target.shell.visit.setWorkspace(citation.workspace);
    open(citation.workspace, { id: citation.file }, citation);
  });

  createEffect(askedProposal, (proposal) => {
    if (proposal === null) return;
    setReview(proposal);
    setWorkspace(proposal.workspace);
    target.shell.visit.setWorkspace(proposal.workspace);
    open(proposal.workspace, { id: proposal.file });
  });

  onSettled(() => () => {
    ++load;
    visible = false;
  });

  return {
    api,
    spaces,
    workspace,
    opened,
    failed,
    selection,
    citation,
    pending,
    remembered,
    recovery,
    repair,
    setRepair,
    choosing,
    reviewing,
    changed,
    revealed,
    progress,
    activate,
    closeFile,
    register,
    save,
    category,
    relink,
    readResource,
    attach,
    useTarget,
    newSession,
    cancelTarget,
    chooseFile,
    link,
    closeReview: () => {
      setReview(null);
    },
    identity: () => {
      const file = opened();
      return file === null ? null : cacheKey(file.workspace, file.record.id);
    },
  };
}

export type WorkspaceDocuments = ReturnType<typeof workspaceDocuments>;

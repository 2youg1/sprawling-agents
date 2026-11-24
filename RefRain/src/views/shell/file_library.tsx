// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal, For, onSettled, Show } from "solid-js";

import { fill, say, type Lang } from "../../core/lang";
import type { WorkspaceClient, WorkspaceEntry, WorkspaceInfo } from "../../core/workspace";
import { WorkspaceFailure } from "./workspace_failure";

export type LibraryTarget = { id: string } | { path: string };

export interface FileLibraryProps {
  readonly api: WorkspaceClient;
  readonly lang: Lang;
  readonly spaces: readonly WorkspaceInfo[];
  readonly workspace: string | null;
  readonly file: string | null;
  readonly path?: string | undefined;
  readonly visible: boolean;
  readonly manage: boolean;
  readonly bindBack: (back: (() => boolean) | undefined) => void;
  readonly onWorkspace: (id: string) => void;
  readonly onRegister: (space: WorkspaceInfo) => void;
  readonly onOpen: (target: LibraryTarget) => void;
}

const FALLBACK_ROW = 44;
const OVERSCAN = 6;

export function FileLibrary(props: FileLibraryProps) {
  const [path, setPath] = createSignal("");
  const [folder, setFolder] = createSignal("");
  const [failure, setFailure] = createSignal<unknown>(null);
  const [entries, setEntries] = createSignal<readonly WorkspaceEntry[]>([]);
  const [next, setNext] = createSignal<string | null>(null);
  const [loading, setLoading] = createSignal(false);
  const [adding, setAdding] = createSignal(false);
  const [offset, setOffset] = createSignal(0);
  const [height, setHeight] = createSignal(0);
  const [rowSize, setRowSize] = createSignal(FALLBACK_ROW);
  let view: HTMLDivElement | undefined;
  let location: HTMLParagraphElement | undefined;
  const positions = new Map<string, { readonly offset: number; readonly count: number }>();
  let request = 0;
  let observation: AbortController | undefined;
  let revision: string | null = null;

  const first = createMemo(() => Math.max(0, Math.floor(offset() / rowSize()) - OVERSCAN));
  const last = createMemo(() =>
    Math.min(entries().length, Math.ceil((offset() + height()) / rowSize()) + OVERSCAN),
  );
  const visibleRows = createMemo(() => entries().slice(first(), last()));
  const recent = createMemo(
    () => props.spaces.find((space) => space.id === props.workspace)?.recent ?? [],
  );
  const trail = createMemo(() => {
    const root = props.spaces.find((space) => space.id === props.workspace)?.name ?? "";
    return path() === "" ? root : `${root} / ${path().split("/").join(" / ")}`;
  });

  function stopWatching(): void {
    observation?.abort();
    observation = undefined;
  }

  function watch(space: string, at: string): void {
    stopWatching();
    if (!props.visible || revision === null) return;
    const observer = new AbortController();
    observation = observer;
    void props.api.observe(space, at, revision, observer.signal).then(
      (notice) => {
        if (observation !== observer) return;
        observation = undefined;
        if (notice.revision === revision) watch(space, at);
        else if (notice.revision !== null) list(space, at, undefined, true);
      },
      () => {
        if (observation === observer) observation = undefined;
      },
    );
  }

  async function loaded(space: string, at: string, cursor: string | undefined, amount: number) {
    const head = await props.api.list(space, at, cursor);
    const all = [...head.entries];
    const seen = new Set<string>();
    let after = head.next;
    while (after !== null && !seen.has(after) && all.length < amount) {
      seen.add(after);
      const following = await props.api.list(space, at, after);
      all.push(...following.entries);
      after = following.next;
    }
    return { entries: all, next: after, revision: head.revision ?? null };
  }

  function list(space: string, at: string, cursor?: string, preserve = false, from?: string): void {
    const id = ++request;
    const saved = positions.get(at);
    const amount = preserve ? entries().length : cursor === undefined ? (saved?.count ?? 0) : 0;
    setLoading(true);
    setFailure(null);
    stopWatching();
    void loaded(space, at, cursor, amount).then(
      (page) => {
        if (id !== request) return;
        setEntries((held) => (cursor === undefined ? page.entries : [...held, ...page.entries]));
        setNext(page.next);
        if (cursor === undefined) revision = page.revision;
        setLoading(false);
        watch(space, at);
        if (!preserve && cursor === undefined)
          requestAnimationFrame(() => {
            if (id !== request || view === undefined) return;
            const into =
              from === undefined ? -1 : page.entries.findIndex((entry) => entry.path === from);
            const top = into < 0 ? (saved?.offset ?? 0) : into * rowSize();
            view.scrollTop = top;
            setOffset(view.scrollTop);
            if (from !== undefined)
              requestAnimationFrame(() => {
                const restored = view?.querySelector<HTMLElement>(
                  `[data-file-path="${CSS.escape(from)}"]`,
                );
                (restored ?? location)?.focus({ preventScroll: true });
              });
          });
      },
      (error: unknown) => {
        if (id !== request) return;
        setLoading(false);
        setFailure(error);
      },
    );
  }

  function enter(space: string | null, at: string, from?: string): void {
    setPath(at);
    setOffset(0);
    setEntries([]);
    setNext(null);
    if (view !== undefined) view.scrollTop = 0;
    if (space === null) {
      ++request;
      stopWatching();
      setLoading(false);
    } else list(space, at, undefined, false, from);
  }

  function rememberFolder(): void {
    positions.set(path(), { offset: view?.scrollTop ?? 0, count: entries().length });
  }

  function descend(at: string): void {
    rememberFolder();
    enter(props.workspace, at);
    requestAnimationFrame(() => location?.focus({ preventScroll: true }));
  }

  function parent(): boolean {
    const at = path();
    if (props.manage || at === "") return false;
    rememberFolder();
    enter(props.workspace, at.includes("/") ? at.slice(0, at.lastIndexOf("/")) : "", at);
    return true;
  }

  function refresh(): void {
    const space = props.workspace;
    if (props.visible && space !== null && !loading()) list(space, path(), undefined, true);
  }

  function more(): void {
    const space = props.workspace;
    const cursor = next();
    if (space !== null && cursor !== null) list(space, path(), cursor);
  }

  function choose(entry: WorkspaceEntry): void {
    if (entry.kind === "directory") descend(entry.path);
    else props.onOpen({ path: entry.path });
  }

  function register(event: SubmitEvent): void {
    event.preventDefault();
    const root = folder().trim();
    if (adding() || root === "") return;
    setAdding(true);
    setFailure(null);
    void props.api.register(root).then(
      (space) => {
        setFolder("");
        setAdding(false);
        props.onRegister(space);
      },
      (error: unknown) => {
        setAdding(false);
        setFailure(error);
      },
    );
  }

  function move(event: KeyboardEvent, index: number): void {
    const node = view;
    if (node === undefined || event.isComposing) return;
    const goal =
      event.key === "ArrowDown"
        ? index + 1
        : event.key === "ArrowUp"
          ? index - 1
          : event.key === "Home"
            ? 0
            : event.key === "End"
              ? entries().length - 1
              : null;
    if (goal === null || goal < 0 || goal >= entries().length) return;
    event.preventDefault();
    const row = rowSize();
    const top = goal * row;
    if (top < node.scrollTop) node.scrollTop = top;
    if (top + row > node.scrollTop + height()) node.scrollTop = top + row - height();
    setOffset(node.scrollTop);
    queueMicrotask(() => {
      node.querySelector<HTMLElement>(`[data-file-index="${String(goal)}"]`)?.focus();
    });
  }

  createEffect(
    () => props.workspace,
    (space) => {
      positions.clear();
      enter(space, "");
    },
  );
  createEffect(
    () => props.visible,
    (visible) => {
      if (visible) refresh();
      else stopWatching();
    },
  );
  onSettled(() => {
    props.bindBack(parent);
    const node = view;
    if (node === undefined) return () => props.bindBack(undefined);
    const measure = () => {
      setHeight(node.clientHeight);
      setRowSize(
        Number.parseFloat(getComputedStyle(node).getPropertyValue("--refrain-file-row")) ||
          FALLBACK_ROW,
      );
    };
    const viewport = new ResizeObserver(measure);
    viewport.observe(node);
    measure();
    window.addEventListener("focus", refresh);
    return () => {
      ++request;
      viewport.disconnect();
      stopWatching();
      window.removeEventListener("focus", refresh);
      props.bindBack(undefined);
    };
  });

  return (
    <>
      <section class="refrain-library refrain-library-manage" hidden={!props.manage}>
        <div class="refrain-files-tools">
          <Show when={props.spaces.length > 0}>
            <label>
              {say(props.lang, "shell_select_workspace")}
              <select
                value={props.workspace ?? ""}
                onChange={(event) => props.onWorkspace(event.currentTarget.value)}
              >
                <For each={props.spaces}>
                  {(space) => <option value={space.id}>{space.name}</option>}
                </For>
              </select>
            </label>
          </Show>
          <Registration
            lang={props.lang}
            folder={folder()}
            adding={adding()}
            onInput={setFolder}
            onRegister={register}
          />
        </div>
        <Show when={failure() !== null}>
          <WorkspaceFailure failure={failure()} lang={props.lang} />
        </Show>
        <Show when={recent().length > 0}>
          <h3>{say(props.lang, "shell_recent_files")}</h3>
          <For each={recent()}>
            {(record) => (
              <button
                class="refrain-file-card"
                data-control="surface"
                type="button"
                aria-current={record.id === props.file ? "true" : undefined}
                onClick={() => props.onOpen({ id: record.id })}
              >
                {record.path} {record.category ?? ""}
              </button>
            )}
          </For>
        </Show>
      </section>
      <section class="refrain-library refrain-library-browse" hidden={props.manage}>
        <Show
          when={props.workspace !== null}
          fallback={<p role="status">{say(props.lang, "shell_empty_library")}</p>}
        >
          <p
            ref={(node) => {
              location = node;
            }}
            tabindex="-1"
            class="refrain-folder-path"
            data-folder-path={path()}
            aria-label={fill(say(props.lang, "shell_current_folder"), { path: trail() })}
            title={trail()}
          >
            {trail()}
          </p>
        </Show>
        <Show when={failure() !== null}>
          <WorkspaceFailure failure={failure()} lang={props.lang} />
        </Show>
        <Show
          when={
            props.workspace !== null && entries().length === 0 && !loading() && failure() === null
          }
        >
          <p role="status">{say(props.lang, "shell_empty_folder")}</p>
        </Show>
        <div
          class="refrain-file-list"
          ref={(node) => {
            view = node;
          }}
          onScroll={(event) => {
            setOffset(event.currentTarget.scrollTop);
          }}
          aria-busy={loading() ? "true" : "false"}
        >
          <div style={{ height: `${entries().length * rowSize()}px` }}>
            <For each={visibleRows()} keyed={(entry) => entry.path}>
              {(entry, index) => (
                <button
                  type="button"
                  class="refrain-file-card absolute overflow-hidden text-ellipsis whitespace-nowrap"
                  data-control="surface"
                  data-file-index={first() + index()}
                  data-file-path={entry().path}
                  data-file-kind={entry().kind}
                  aria-current={entry().path === props.path ? "true" : undefined}
                  style={{ top: `${(first() + index()) * rowSize()}px`, height: `${rowSize()}px` }}
                  onClick={() => choose(entry())}
                  onKeyDown={(event) => move(event, first() + index())}
                >
                  {entry().name}
                  <Show when={entry().kind === "directory"}>
                    <span aria-hidden="true">/</span>
                  </Show>
                </button>
              )}
            </For>
          </div>
        </div>
        <div class="refrain-directory-actions">
          <Show when={path() !== ""}>
            <button
              class="refrain-nav-button"
              data-control="surface"
              type="button"
              onClick={parent}
            >
              {say(props.lang, "shell_parent_folder")}
            </button>
          </Show>
          <Show when={next() !== null && props.workspace !== null}>
            <button
              class="refrain-nav-button"
              data-control="surface"
              type="button"
              disabled={loading()}
              onClick={more}
            >
              {say(props.lang, "shell_next_files")}
            </button>
          </Show>
        </div>
      </section>
    </>
  );
}

function Registration(props: {
  readonly lang: Lang;
  readonly folder: string;
  readonly adding: boolean;
  readonly onInput: (folder: string) => void;
  readonly onRegister: (event: SubmitEvent) => void;
}) {
  return (
    <form
      class="refrain-files-tools refrain-file-registration"
      onSubmit={(event) => props.onRegister(event)}
    >
      <label>
        {say(props.lang, "shell_workspace_root")}
        <input
          value={props.folder}
          onInput={(event) => props.onInput(event.currentTarget.value)}
          required
        />
      </label>
      <button
        class="refrain-nav-button"
        data-control="surface"
        type="submit"
        disabled={props.adding}
      >
        {say(props.lang, "shell_register_workspace")}
      </button>
    </form>
  );
}

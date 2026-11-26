// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import {
  closeSearchPanel,
  findNext,
  findPrevious,
  getSearchQuery,
  openSearchPanel,
  replaceAll,
  replaceNext,
  SearchQuery,
  setSearchQuery,
} from "@codemirror/search";
import { runScopeHandlers, type Command, type EditorView } from "@codemirror/view";
import { createMemo, createSignal, onSettled, Show } from "solid-js";

import { fill, say, type Key, type Lang } from "../../core/lang";
import Glyph from "../parts/glyph.solid";

const COUNT_LIMIT = 2000;

export function WorkspaceSearch(props: { readonly view: EditorView; readonly lang: Lang }) {
  const word = (key: Key) => say(props.lang, key);
  const [held, setHeld] = createSignal(getSearchQuery(props.view.state));
  const [revision, bump] = createSignal(0);
  let field: HTMLInputElement | undefined;

  function change(next: Partial<ConstructorParameters<typeof SearchQuery>[0]>): void {
    const { search, replace, caseSensitive, literal, regexp, wholeWord } = held();
    const query = new SearchQuery({
      search,
      replace,
      caseSensitive,
      literal,
      regexp,
      wholeWord,
      ...next,
    });
    setHeld(query);
    props.view.dispatch({ effects: setSearchQuery.of(query) });
  }

  function command(run: Command): void {
    run(props.view);
    bump((value) => value + 1);
  }

  function replaceCurrent(view: EditorView): boolean {
    const { from, to } = view.state.selection.main;
    const match = held().getCursor(view.state, from, to).next();
    if (match.done || match.value.from !== from || match.value.to !== to) {
      if (!findNext(view)) return false;
    }
    return replaceNext(view);
  }

  const matches = createMemo(() => {
    revision();
    const query = held();
    if (!query.valid) return { total: 0, current: 0, more: false };
    const { state } = props.view;
    const { from, to } = state.selection.main;
    const cursor = query.getCursor(state);
    let total = 0;
    let current = 0;
    let next = cursor.next();
    while (!next.done && total < COUNT_LIMIT) {
      total += 1;
      if (next.value.from === from && next.value.to === to) current = total;
      next = cursor.next();
    }
    return { total, current, more: !next.done };
  });
  const count = () => {
    const { total, current, more } = matches();
    if (held().search === "") return "";
    const size = String(total) + (more ? "+" : "");
    if (total === 0) return word("writing_search_empty");
    return current === 0
      ? fill(word("writing_search_total"), { total: size })
      : fill(word("writing_search_count"), { current: String(current), total: size });
  };

  onSettled(() => {
    openSearchPanel(props.view);
    setHeld(getSearchQuery(props.view.state));
    field?.focus({ preventScroll: true });
    field?.select();
    return () => {
      if (props.view.dom.isConnected) closeSearchPanel(props.view);
    };
  });

  return (
    <div
      class="workspace-edit-panel"
      onKeyDown={(event) => {
        if (event.defaultPrevented || event.isComposing) return;
        if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "f") {
          event.preventDefault();
          field?.focus({ preventScroll: true });
          field?.select();
        } else if (runScopeHandlers(props.view, event, "search-panel")) {
          event.preventDefault();
          bump((value) => value + 1);
        }
      }}
    >
      <label class="workspace-edit-field">
        <span>{word("refrain_cm_find")}</span>
        <input
          type="search"
          data-control=""
          data-context-focus=""
          ref={(node) => {
            field = node;
          }}
          value={held().search}
          onInput={(event) => change({ search: event.currentTarget.value })}
          onKeyDown={(event) => {
            if (event.key !== "Enter" || event.isComposing) return;
            event.preventDefault();
            command(event.shiftKey ? findPrevious : findNext);
          }}
        />
      </label>
      <div class="workspace-edit-actions">
        <output role="status" class="grow">
          {held().search !== "" && !held().valid ? word("writing_search_invalid") : count()}
        </output>
        <button
          type="button"
          data-control="surface"
          aria-label={word("refrain_cm_previous")}
          disabled={matches().total === 0}
          onClick={() => command(findPrevious)}
        >
          <Glyph name="up" size="sm" />
        </button>
        <button
          type="button"
          data-control="surface"
          aria-label={word("refrain_cm_next")}
          disabled={matches().total === 0}
          onClick={() => command(findNext)}
        >
          <Glyph name="up" size="sm" class="rotate-180" />
        </button>
      </div>
      <div class="workspace-edit-actions">
        <label>
          <input
            type="checkbox"
            data-control=""
            checked={held().caseSensitive}
            onChange={(event) =>
              change({ search: held().search, caseSensitive: event.currentTarget.checked })
            }
          />
          {word("refrain_cm_match_case")}
        </label>
        <label>
          <input
            type="checkbox"
            data-control=""
            checked={held().regexp}
            onChange={(event) =>
              change({ search: held().search, regexp: event.currentTarget.checked })
            }
          />
          {word("refrain_cm_regexp")}
        </label>
        <label>
          <input
            type="checkbox"
            data-control=""
            checked={held().wholeWord}
            onChange={(event) =>
              change({ search: held().search, wholeWord: event.currentTarget.checked })
            }
          />
          {word("refrain_cm_by_word")}
        </label>
      </div>
      <Show when={!props.view.state.readOnly}>
        <label class="workspace-edit-field">
          <span>{word("refrain_cm_replace")}</span>
          <input
            type="text"
            data-control=""
            value={held().replace}
            onInput={(event) =>
              change({ search: held().search, replace: event.currentTarget.value })
            }
          />
        </label>
        <div class="workspace-edit-actions justify-end">
          <button
            type="button"
            data-control="surface"
            disabled={matches().total === 0}
            onClick={() => command(replaceCurrent)}
          >
            {word("refrain_cm_replace_one")}
          </button>
          <button
            type="button"
            data-control="surface"
            disabled={matches().total === 0}
            onClick={() => command(replaceAll)}
          >
            {word("refrain_cm_replace_all")}
          </button>
        </div>
      </Show>
    </div>
  );
}

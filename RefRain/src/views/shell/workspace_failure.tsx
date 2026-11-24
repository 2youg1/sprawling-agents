// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal, onCleanup, Show } from "solid-js";

import { say, type Lang } from "../../core/lang";
import { WorkspaceError } from "../../core/workspace";
import { lookOf, presser, REST, type Copied } from "../parts/copy";
import Glyph from "../parts/glyph.solid";
import { errorText, workspaceDiagnostic } from "./workspace_error";

export function WorkspaceFailure(props: { readonly failure: unknown; readonly lang: Lang }) {
  const [copied, setCopied] = createSignal<Copied>(REST);
  const diagnostic = createMemo(() => {
    const failure = props.failure;
    return failure instanceof WorkspaceError ? failure.diagnostic : undefined;
  });
  const copyText = createMemo(() => {
    const value = diagnostic();
    return value === undefined ? "" : workspaceDiagnostic(value);
  });
  let waiting: ReturnType<typeof setTimeout> | undefined;
  onCleanup(() => clearTimeout(waiting));
  const copy = presser({
    board: () => navigator.clipboard,
    later: (run, ms) => {
      const handle = setTimeout(run, ms);
      waiting = handle;
      return () => clearTimeout(handle);
    },
    show: setCopied,
  });
  const look = createMemo(() =>
    lookOf(
      { text: copyText, note: say(props.lang, "shell_copy_diagnostic") },
      copied(),
      props.lang,
      () => copy(copyText()),
    ),
  );
  return (
    <>
      <p role="alert">{errorText(props.failure)}</p>
      <Show when={diagnostic()}>
        {(failure) => (
          <details>
            <summary>{failure().code}</summary>
            <pre>{copyText()}</pre>
            <button
              data-control=""
              class="refrain-nav-button"
              type="button"
              title={look().note}
              onClick={() => look().key.onclick()}
            >
              <Glyph name={look().face} size="sm" /> {look().name}
            </button>
            <span class="sr-only" role="status">
              {look().said}
            </span>
          </details>
        )}
      </Show>
    </>
  );
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What moved between two checkpoints: the files as rows, and under the
// row a hand opened, the patch of that one file. Drawn the same from a
// run's lens and from a building's commit list, so one reading of a
// diff exists.

import { For, Show, createMemo, createSignal } from "solid-js";

import type { FileChange, GitOid } from "../wire";
import { useSay, useUi } from "../ui";

export function Hunks(props: { readonly base: GitOid; readonly head: GitOid; readonly path: string }) {
  const ui = useUi();
  const say = useSay();
  const hunks = createMemo(() => ui.conn.asking.ask({ hunks: { oid_a: props.base, oid_b: props.head, path: props.path } }));
  const answer = createMemo(() => {
    const held = hunks()();
    return held !== undefined && "hunks" in held ? held.hunks : undefined;
  });
  return (
    <Show when={answer()} fallback={<p class="text-text-disabled">…</p>}>
      {(patch) => (
        <pre class="overflow-x-auto rounded-card bg-g1 p-base font-mono text-note leading-relaxed">
          <For each={patch().lines}>
            {(line) => (
              <div class={line.text.startsWith("+") ? "text-accent" : line.text.startsWith("-") ? "text-alert" : "text-text-quiet"}>
                {line.text}
              </div>
            )}
          </For>
          <For each={patch().withheld}>
            {(held) => (
              <div class="text-text-disabled">
                {say("run_withheld", { n: String(held.number), reason: held.reason })}
              </div>
            )}
          </For>
        </pre>
      )}
    </Show>
  );
}

export function Changes(props: { readonly base: GitOid; readonly head: GitOid | null }) {
  const ui = useUi();
  const say = useSay();
  const [open, setOpen] = createSignal<string | null>(null);
  const changes = createMemo(() => ui.conn.asking.ask({ changes: { base: props.base, head: props.head } }));
  const files = createMemo<readonly FileChange[] | undefined>(() => {
    const held = changes()();
    return held !== undefined && "changes" in held ? held.changes.files : undefined;
  });
  const how = (file: FileChange) => (typeof file.how === "string" ? say(`change_${file.how}`) : say("change_renamed", { from: file.how.renamed.from }));
  return (
    <Show when={files()} fallback={<p class="text-text-disabled">…</p>}>
      {(held) => (
        <Show when={held().length > 0} fallback={<p class="text-text-faint">{say("run_no_changes")}</p>}>
          <ul class="text-note">
            <For each={held()}>
              {(file) => (
                <li class="border-b border-g1">
                  <button
                    type="button"
                    class="flex w-full items-center gap-base py-snug text-left hover:text-text"
                    aria-expanded={open() === file.path}
                    onClick={() => setOpen((at) => (at === file.path ? null : file.path))}
                  >
                    <span class="w-figure shrink-0 text-text-faint">{how(file)}</span>
                    <span class="flex-1 truncate font-mono text-text-quiet">{file.path}</span>
                    <span class="shrink-0 font-mono text-text-disabled">
                      {typeof file.lines === "string" ? say("change_binary") : `+${String(file.lines.counted.added)} −${String(file.lines.counted.removed)}`}
                    </span>
                  </button>
                  <Show when={open() === file.path}>
                    <Show when={props.head} fallback={<p class="pb-base text-text-disabled">{say("run_patch_needs_fence")}</p>}>
                      {(head) => (
                        <div class="pb-base">
                          <Hunks base={props.base} head={head()} path={file.path} />
                        </div>
                      )}
                    </Show>
                  </Show>
                </li>
              )}
            </For>
          </ul>
        </Show>
      )}
    </Show>
  );
}

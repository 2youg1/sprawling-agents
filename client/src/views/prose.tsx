// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Draws what `core/prose` read: blocks and inline marks as elements,
// never as HTML. Text nodes only, so a model's words cannot become
// markup.

import { For, Match, Switch, createMemo } from "solid-js";

import { blocks, inline as inlineOf } from "../core/prose";
import type { Block, Inline } from "../core/prose";

function Inlines(props: { readonly parts: readonly Inline[] }) {
  return (
    <For each={props.parts}>
      {(part) => (
        <Switch>
          <Match when={part.kind === "text" ? part : undefined}>{(p) => p().text}</Match>
          <Match when={part.kind === "code" ? part : undefined}>
            {(p) => <code class="rounded-control bg-g2 px-tight font-mono text-note">{p().text}</code>}
          </Match>
          <Match when={part.kind === "strong" ? part : undefined}>
            {(p) => <strong class="font-label">{p().text}</strong>}
          </Match>
          <Match when={part.kind === "em" ? part : undefined}>{(p) => <em>{p().text}</em>}</Match>
          <Match when={part.kind === "link" ? part : undefined}>
            {(p) => (
              <a href={p().href} rel="noreferrer" target="_blank" class="text-accent underline">
                {p().text}
              </a>
            )}
          </Match>
        </Switch>
      )}
    </For>
  );
}

function BlockView(props: { readonly block: Block }) {
  return (
    <Switch>
      <Match when={props.block.kind === "paragraph" ? props.block : undefined}>
        {(b) => (
          <p class="my-snug leading-relaxed">
            <Inlines parts={b().inline} />
          </p>
        )}
      </Match>
      <Match when={props.block.kind === "heading" ? props.block : undefined}>
        {(b) => (
          <p class={`mt-base mb-tight font-heading ${b().level <= 2 ? "text-heading" : "text-body"}`}>
            <Inlines parts={b().inline} />
          </p>
        )}
      </Match>
      <Match when={props.block.kind === "list" ? props.block : undefined}>
        {(b) => (
          <ul class={`my-snug pl-wide ${b().ordered ? "list-decimal" : "list-disc"}`}>
            <For each={b().items}>
              {(item) => (
                <li class="my-tight leading-relaxed">
                  <Inlines parts={item} />
                </li>
              )}
            </For>
          </ul>
        )}
      </Match>
      <Match when={props.block.kind === "code" ? props.block : undefined}>
        {(b) => (
          <pre class="my-snug overflow-x-auto rounded-card bg-g1 p-base font-mono text-note leading-relaxed text-text-quiet">
            {b().text}
          </pre>
        )}
      </Match>
      <Match when={props.block.kind === "quote" ? props.block : undefined}>
        {(b) => (
          <blockquote class="my-snug border-l-2 border-g4 pl-base text-text-quiet">
            <Inlines parts={b().inline} />
          </blockquote>
        )}
      </Match>
      <Match when={props.block.kind === "table" ? props.block : undefined}>
        {(b) => (
          <div class="my-snug overflow-x-auto">
            <table class="w-full border-collapse text-note">
              <tbody>
                <For each={b().rows}>
                  {(row, index) => (
                    <tr class={index() === 0 ? "text-text-quiet" : ""}>
                      <For each={row}>
                        {(cell) => (
                          <td class="border-b border-g2 px-snug py-tight align-top">
                            <Inlines parts={inlineOf(cell)} />
                          </td>
                        )}
                      </For>
                    </tr>
                  )}
                </For>
              </tbody>
            </table>
          </div>
        )}
      </Match>
    </Switch>
  );
}


export function Prose(props: { readonly text: string }) {
  const parsed = createMemo(() => blocks(props.text));
  return (
    <div class="prose-region">
      <For each={parsed()}>{(block) => <BlockView block={block} />}</For>
    </div>
  );
}

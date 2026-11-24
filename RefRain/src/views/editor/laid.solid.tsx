// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { dynamic, type JSX } from "@solidjs/web";
import { createMemo, For, Show } from "solid-js";

import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Block, Inline } from "../../wire";
import Inked from "../parts/inked.solid";
import Formula from "./formula.solid";

export interface LaidProps {
  readonly blocks: readonly Block[];
  readonly onOpen?: ((target: string) => void) | undefined;
  readonly placed?: boolean;
}

type Open = LaidProps["onOpen"];
type List = Extract<Block, { readonly list: unknown }>["list"];
type Table = Extract<Block, { readonly table: unknown }>["table"];

const ABSOLUTE = /^(?:https?|mailto):/iu;
const SPACE = " ";
const ALIGN: Readonly<Record<string, string>> = {
  none: "text-left",
  left: "text-left",
  center: "text-center",
  right: "text-right",
};

function Inlines(props: { readonly parts: readonly Inline[]; readonly onOpen: Open }) {
  return (
    <For each={props.parts} keyed={false}>
      {(part) => {
        const mark = createMemo(() => inline(part(), props.onOpen));
        return <>{mark()}</>;
      }}
    </For>
  );
}

function inline(part: Inline, onOpen: Open): JSX.Element {
  if (part === "soft_break") return SPACE;
  if (part === "line_break") return <br />;
  if ("text" in part) return part.text;
  if ("code" in part) {
    return <code class="rounded-control bg-raised px-tight font-mono text-note">{part.code}</code>;
  }
  if ("emphasis" in part)
    return (
      <em>
        <Inlines parts={part.emphasis} onOpen={onOpen} />
      </em>
    );
  if ("strong" in part) {
    return (
      <strong class="font-label">
        <Inlines parts={part.strong} onOpen={onOpen} />
      </strong>
    );
  }
  if ("strikethrough" in part)
    return (
      <s>
        <Inlines parts={part.strikethrough} onOpen={onOpen} />
      </s>
    );
  if ("link" in part) {
    const { target, content } = part.link;
    if (ABSOLUTE.test(target)) {
      return (
        <a href={target} rel="noreferrer" target="_blank" class="text-accent underline">
          <Inlines parts={content} onOpen={onOpen} />
        </a>
      );
    }
    if (onOpen === undefined) return <Inlines parts={content} onOpen={onOpen} />;
    return (
      <button
        data-control="surface"
        type="button"
        class="text-accent underline"
        onClick={() => onOpen(target)}
      >
        <Inlines parts={content} onOpen={onOpen} />
      </button>
    );
  }
  if ("image" in part) return <span class="text-text-faint">{part.image.alt}</span>;
  if ("footnote_reference" in part)
    return <sup class="text-text-faint">{part.footnote_reference.name}</sup>;
  const { construct, source } = part.unsupported;
  return construct === "math" ? (
    <Formula source={source} />
  ) : (
    <code class="font-mono text-note text-text-faint">{source}</code>
  );
}

function ListBlock(props: {
  readonly list: List;
  readonly start: number | undefined;
  readonly onOpen: Open;
}) {
  const lang = storeValue(ui().lang);
  const order = () => props.list.order;
  const ordered = () => {
    const value = order();
    return value === "bullet" ? undefined : value.ordered.start;
  };
  const Container = dynamic(() => (order() === "bullet" ? "ul" : "ol"));
  return (
    <Container
      data-start={props.start}
      start={ordered()}
      class={[
        "my-snug",
        order() === "bullet"
          ? "pl-wide list-disc"
          : "pl-[calc(var(--spacing-wide)+2ch)] list-decimal",
      ]}
    >
      <For each={props.list.items} keyed={false}>
        {(item) => (
          <li
            class={[
              "leading-read",
              props.list.spacing === "loose" ? "my-snug" : "my-tight [&_p]:my-0",
              item().check === "not_a_task"
                ? ""
                : "-ml-wide flex list-none items-baseline gap-snug",
            ]}
          >
            <Show when={item().check !== "not_a_task"}>
              <input
                type="checkbox"
                disabled
                checked={item().check === "done"}
                aria-label={say(
                  lang(),
                  item().check === "done" ? "refrain_task_done" : "refrain_task_open",
                )}
                class="w-glyph-sm shrink-0"
              />
            </Show>
            <div class="min-w-0 flex-1">
              <Laid blocks={item().blocks} onOpen={props.onOpen} />
            </div>
          </li>
        )}
      </For>
    </Container>
  );
}

function TableBlock(props: {
  readonly table: Table;
  readonly start: number | undefined;
  readonly onOpen: Open;
}) {
  const alignment = (at: number) => ALIGN[props.table.align[at] ?? "none"];
  return (
    <div data-start={props.start} class="my-snug overflow-x-auto">
      <table class="w-full border-collapse text-note">
        <thead>
          <tr class="text-text-quiet">
            <For each={props.table.head.cells} keyed={false}>
              {(cell, at) => (
                <th
                  class={["border-b border-edge-input px-snug py-tight font-label", alignment(at)]}
                >
                  <Inlines parts={cell()} onOpen={props.onOpen} />
                </th>
              )}
            </For>
          </tr>
        </thead>
        <tbody>
          <For each={props.table.body} keyed={false}>
            {(row) => (
              <tr>
                <For each={row().cells} keyed={false}>
                  {(cell, at) => (
                    <td class={["border-b border-edge px-snug py-tight align-top", alignment(at)]}>
                      <Inlines parts={cell()} onOpen={props.onOpen} />
                    </td>
                  )}
                </For>
              </tr>
            )}
          </For>
        </tbody>
      </table>
    </div>
  );
}

function blockView(block: Block, placed: boolean, onOpen: Open): JSX.Element {
  const start = (span: { readonly start: number }) => (placed ? span.start : undefined);
  if ("heading" in block) {
    const heading = block.heading;
    return (
      <p
        data-start={start(heading.span)}
        class={["mt-base mb-tight font-heading", heading.level <= 2 ? "text-heading" : "text-body"]}
      >
        <Inlines parts={heading.inline} onOpen={onOpen} />
      </p>
    );
  }
  if ("paragraph" in block) {
    return (
      <p data-start={start(block.paragraph.span)} class="my-snug leading-read">
        <Inlines parts={block.paragraph.inline} onOpen={onOpen} />
      </p>
    );
  }
  if ("list" in block)
    return <ListBlock list={block.list} start={start(block.list.span)} onOpen={onOpen} />;
  if ("quote" in block) {
    return (
      <blockquote
        data-start={start(block.quote.span)}
        class="my-snug border-l-2 border-edge-input pl-base text-text-quiet"
      >
        <Laid blocks={block.quote.blocks} onOpen={onOpen} />
      </blockquote>
    );
  }
  if ("code" in block) {
    return (
      <pre
        data-start={start(block.code.span)}
        class="my-snug overflow-x-auto rounded-card border border-edge bg-page p-base font-mono text-note leading-read text-text-quiet"
      >
        <Inked text={block.code.text} source={block.code.info} />
      </pre>
    );
  }
  if ("table" in block)
    return <TableBlock table={block.table} start={start(block.table.span)} onOpen={onOpen} />;
  if ("rule" in block)
    return <hr data-start={start(block.rule.span)} class="my-base border-edge" />;
  if ("footnote" in block) {
    const note = block.footnote;
    return (
      <div
        data-start={start(note.span)}
        class="my-snug flex items-baseline gap-snug text-note text-text-quiet"
      >
        <sup class="text-text-faint">{note.name}</sup>
        <div class="min-w-0 flex-1">
          <Laid blocks={note.blocks} onOpen={onOpen} />
        </div>
      </div>
    );
  }
  return (
    <pre
      data-start={start(block.unsupported.span)}
      class="my-snug overflow-x-auto whitespace-pre-wrap font-mono text-note text-text-faint"
    >
      {block.unsupported.source}
    </pre>
  );
}

export default function Laid(props: LaidProps) {
  return (
    <div class="contents font-read">
      <For each={props.blocks} keyed={false}>
        {(block) => {
          const drawn = createMemo(() => blockView(block(), props.placed === true, props.onOpen));
          return <>{drawn()}</>;
        }}
      </For>
    </div>
  );
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Show, createMemo } from "solid-js";
import { fill, say, type Lang } from "../../core/lang";
import type { Tail } from "../../core/live_output";
import { storeValue } from "../../core/solid_store";
import { lasted } from "../../core/time";
import { ui } from "../../ui";
import Glyph from "../parts/glyph.solid";
import type { GlyphName } from "../parts/glyph";
import Inked from "../parts/inked.solid";
import { INK, rows } from "./ansi";
import Ceilings from "./ceilings.solid";
import { lookOf, type CodeColumnLook } from "./code_column";
import type { Ending, Entry, Sign, Touched } from "./trace";
import "./trace_view.css";

const ACTION =
  "inline-flex h-control-sm items-center rounded-control px-snug text-label text-text-quiet transition-colors ease-leave hover:bg-raised hover:text-text hover:ease-arrive";
const ROW: Record<Sign, string> = { added: "bg-accent/10", removed: "bg-alert/12", kept: "" };
const SIGN: Record<Sign, string> = { added: "+", removed: "−", kept: "" };
const SIGN_INK: Record<Sign, string> = {
  added: "text-accent",
  removed: "text-alert",
  kept: "text-text-faint",
};

export function CodeColumn(props: {
  readonly files: readonly Touched[];
  readonly live: boolean;
  readonly onDraft: (text: string) => void;
  readonly onSteer: (text: string) => void;
}) {
  const u = ui();
  const lang = storeValue(u.lang);
  const look = createMemo(() =>
    lookOf({ files: props.files, live: props.live, editor: u.prefs.editor() }, lang(), {
      draft: props.onDraft,
      steer: props.onSteer,
    }),
  );
  return <CodeColumnView look={look()} />;
}

export function CodeColumnView(props: { readonly look: CodeColumnLook }) {
  return (
    <>
      <Show when={props.look.empty}>
        <p class="px-snug py-snug text-note text-text-faint">{props.look.empty}</p>
      </Show>
      <For each={props.look.files} keyed={(file) => file.path}>
        {(file) => (
          <div data-path={file().path}>
            <div class="sticky top-0 z-(--layer-raised) flex h-control items-center gap-snug border-b border-edge bg-chrome px-snug text-note">
              <span class="min-w-0 truncate font-mono text-text">{file().path}</span>
              <span class="shrink-0 rounded-pill bg-raised px-snug text-text-quiet">
                {file().how}
              </span>
            </div>
            <For each={file().hunks}>
              {(hunk) => (
                <>
                  <Show when={hunk.actions.length > 0}>
                    <div class="flex justify-end gap-tight border-b border-edge bg-page px-snug">
                      <For each={hunk.actions}>
                        {(action) =>
                          action.kind === "press" ? (
                            <button
                              type="button"
                              data-control="surface"
                              class={ACTION}
                              onClick={() => action.wire.onclick()}
                            >
                              {action.label}
                            </button>
                          ) : (
                            <a class={ACTION} href={action.wire.href}>
                              {action.label}
                            </a>
                          )
                        }
                      </For>
                    </div>
                  </Show>
                  <div class="run-trace-hunk border-b border-edge bg-page font-mono text-note">
                    <For each={hunk.lines}>
                      {(line) => (
                        <div class={["flex", ROW[line.sign]]}>
                          <span
                            class="flex shrink-0 select-none text-text-faint figure"
                            aria-hidden="true"
                          >
                            <span class="run-trace-line-number pe-tight text-right">
                              {line.old}
                            </span>
                            <span class="run-trace-line-number pe-tight text-right">
                              {line.new}
                            </span>
                            <span class={["run-trace-sign text-center", SIGN_INK[line.sign]]}>
                              {SIGN[line.sign]}
                            </span>
                          </span>
                          <span class="run-trace-text min-w-0 flex-1 pe-snug whitespace-pre-wrap wrap-break-word text-text-quiet">
                            <Inked text={line.text} source={file().path} />
                          </span>
                        </div>
                      )}
                    </For>
                  </div>
                </>
              )}
            </For>
          </div>
        )}
      </For>
    </>
  );
}

const MARK: Record<Ending["kind"], GlyphName> = {
  code: "check",
  stopped: "cross",
  running: "pulse",
  unread: "ring",
};

function ended(lang: Lang, ending: Ending): string {
  switch (ending.kind) {
    case "code":
      return fill(say(lang, "mon_exited"), { code: String(ending.code) });
    case "stopped":
      return fill(say(lang, "mon_stopped"), { why: ending.why });
    case "running":
      return say(lang, "mon_running");
    case "unread":
      return say(lang, "mon_unread");
  }
}

function Lines(props: { readonly streams: readonly (readonly [string, boolean])[] }) {
  const drawn = createMemo(() =>
    props.streams.flatMap(([text, alert]) => rows(text).map((line) => ({ alert, line }))),
  );
  return (
    <For each={drawn()}>
      {(item) => (
        <div class="ps-[2ch] -indent-[2ch] whitespace-pre-wrap wrap-break-word">
          <For each={item.line}>
            {(piece) => (
              <span
                class={[item.alert ? "text-alert" : INK[piece.tone], piece.bold && "font-semibold"]}
              >
                {piece.text}
              </span>
            )}
          </For>
        </div>
      )}
    </For>
  );
}

function Command(props: {
  readonly item: Extract<Entry, { kind: "command" }>;
  readonly tail: Tail;
  readonly lang: Lang;
}) {
  const running = () => props.item.ending.kind === "running";
  const failed = () => {
    const ending = props.item.ending;
    return ending.kind === "stopped" || (ending.kind === "code" && ending.code !== 0);
  };
  const cut = () => (running() ? props.tail.cut : props.item.cut);
  return (
    <>
      <div class={["flex min-h-control items-center gap-snug px-snug", failed() && "bg-alert/12"]}>
        <span class="shrink-0 text-text-faint" aria-hidden="true">
          $
        </span>
        <span class="min-w-0 truncate text-text">{props.item.text}</span>
        <span
          class={[
            "ms-auto flex shrink-0 items-center gap-tight ps-snug",
            failed() ? "text-alert" : running() ? "text-accent" : "text-text-faint",
          ]}
        >
          <Glyph name={MARK[props.item.ending.kind]} size="sm" />
          <span>{ended(props.lang, props.item.ending)}</span>
          <Show when={props.item.took !== null}>
            <span class="text-text-faint">{lasted(props.item.took ?? 0)}</span>
          </Show>
        </span>
      </div>
      <div class="px-snug">
        <Lines
          streams={
            running()
              ? [
                  [props.tail.out, false],
                  [props.tail.err, true],
                ]
              : [
                  [props.item.stdout, false],
                  [props.item.stderr, true],
                ]
          }
        />
        <Show when={cut() > 0}>
          <div class="text-text-faint">
            {fill(say(props.lang, "mon_lines_cut"), { n: String(cut()) })}
          </div>
        </Show>
        <Ceilings notes={props.item.ceilings} />
      </div>
    </>
  );
}

export function Terminal(props: { readonly entries: readonly Entry[]; readonly tail: Tail }) {
  const lang = storeValue(ui().lang);
  return (
    <>
      <Show when={props.entries.length === 0}>
        <p class="px-snug py-snug text-note text-text-faint">{say(lang(), "mon_nothing_ran")}</p>
      </Show>
      <For each={props.entries} keyed={(entry) => entry.at}>
        {(entry) => {
          const cmd = () => {
            const value = entry();
            return value.kind === "command" ? value : undefined;
          };
          const call = () => {
            const value = entry();
            return value.kind === "call" ? value : undefined;
          };
          return (
            <div
              data-at={entry().at}
              class="border-b border-edge py-tight font-mono text-note leading-[1.6]"
            >
              <Show when={cmd()}>
                {(item) => <Command item={item()} tail={props.tail} lang={lang()} />}
              </Show>
              <Show when={call()}>
                {(item) => (
                  <>
                    <div class="flex min-h-control items-center gap-snug px-snug">
                      <span class="shrink-0 text-accent">{item().tool}</span>
                      <span class="min-w-0 truncate text-text-quiet">{item().subject}</span>
                      <Show when={item().outcome === "failed"}>
                        <Glyph name="cross" size="sm" class="ms-auto shrink-0 text-alert" />
                      </Show>
                    </div>
                    <div class="px-snug text-text-faint">
                      <Lines
                        streams={[
                          [item().asked, false],
                          [item().answered, item().outcome === "failed"],
                        ]}
                      />
                    </div>
                  </>
                )}
              </Show>
            </div>
          );
        }}
      </For>
    </>
  );
}

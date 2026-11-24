// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal, createUniqueId, Show } from "solid-js";
import { urgencyOf } from "../../core/deferral";
import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { markEndAtFrame, markStart } from "../../core/timing";
import { ui } from "../../ui";
import { recover } from "../notice_recovery";
import Button from "../parts/button.solid";
import ControlTip from "../parts/control_tip";
import Glyph from "../parts/glyph.solid";
import { focusEdgeKey, MAILBOX_KEY } from "../shell/chrome";
import { linkOf } from "./column";
import Deciding from "./deciding";
import { openCards } from "./deciding_proposals";
import { walk } from "./entries";
import { mailboxKey } from "./key";
import { stepLetter, stepMail, type MailFocus, type MailInput } from "./layer";
import Notices from "./notices";
import Recent from "./recent";
import { takeRow, wantedRows } from "./returning";
import Working from "./working";
import "./mailbox.css";

export interface MailboxProps {
  readonly asked: number;
  readonly hint: (words: string) => string;
  readonly keyId?: string;
}

export function MailboxColumn(props: {
  readonly onClose: () => void;
  readonly onBack?: () => void;
}) {
  const u = ui();
  const lang = storeValue(u.lang);
  const connection = storeValue(u.conn.state);
  const [scroller, setScroller] = createSignal<HTMLElement>();
  const link = createMemo(() =>
    linkOf(connection(), lang(), (lever, error) => {
      recover(u, lever, { error, about: null });
    }),
  );
  return (
    <div class="flex h-full min-h-0 flex-col">
      <header class="flex h-bar shrink-0 items-center gap-snug border-b border-edge px-base">
        <span class="hidden narrow:inline-flex">
          <Button
            tone="quiet"
            label={say(lang(), "mailbox_back")}
            onPress={props.onBack ?? props.onClose}
          />
        </span>
        <h2 class="min-w-0 flex-1 truncate text-label font-label">{say(lang(), "edge_mailbox")}</h2>
        <Show when={link()}>
          {(held) => (
            <>
              <span class="shrink-0 text-note text-text-faint">{held().word}</span>
              <Show when={held().recovery}>
                {(action) => (
                  <Button tone="quiet" label={action().label} onPress={action().onPress} />
                )}
              </Show>
            </>
          )}
        </Show>
      </header>
      <div
        class="mailbox-entries min-h-0 flex-1 overflow-y-auto px-base pb-wide"
        ref={(node) => {
          setScroller(node);
        }}
        onKeyDown={(event) => {
          if (!event.defaultPrevented) walk(event.currentTarget, event);
        }}
      >
        <Deciding onLeave={props.onClose} />
        <Working onLeave={props.onClose} />
        <Recent scroller={scroller} onLeave={props.onClose} />
        <Notices />
      </div>
    </div>
  );
}

export default function Mailbox(props: MailboxProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const link = storeValue(u.conn.state);
  const approvals = storeValue(u.approvals);
  const wanted = storeValue(wantedRows);
  const proposed = storeValue(openCards(u.conn.asking));
  const [open, setOpen] = createSignal(false);
  let column: HTMLElement | undefined;
  let key: HTMLButtonElement | undefined;
  let heard = 0;
  const uid = createUniqueId();
  const aside = `${uid}-column`;
  const hint = `${uid}-hint`;
  const keyId = () => props.keyId ?? MAILBOX_KEY;
  const blocking = createMemo(
    () =>
      belief().notices.filter((notice) => !notice.seen && urgencyOf(notice.error) === "needs_you")
        .length,
  );
  const needs = createMemo(() => (approvals()?.length ?? 0) + blocking() + proposed().length);
  const fresh = createMemo(() =>
    belief().notices.some((notice) => !notice.seen && urgencyOf(notice.error) === "ordinary"),
  );
  const look = createMemo(() =>
    mailboxKey({ needs: needs(), fresh: fresh(), link: link() }, lang(), props.hint, {
      "aria-expanded": open(),
      "aria-controls": aside,
      onclick: () => {
        feed("toggle");
      },
    }),
  );
  const tally = createMemo(() => {
    const corner = look().corner;
    return corner.kind === "count" ? corner.text : null;
  });
  const bar = createMemo(() => {
    const foot = look().foot;
    return foot.kind === "link" ? foot.link : null;
  });

  function focusNow(): MailFocus {
    const active = document.activeElement;
    if (active !== null && column?.contains(active) === true) return "inside";
    return active === key ? "key" : "elsewhere";
  }
  function feed(input: MailInput): void {
    const focus = focusNow();
    const next = stepMail({ shown: open(), focus }, input);
    const opening = next.shown && !open();
    if (opening) {
      markStart("mailbox_open");
      u.conn.markNoticesSeen();
    }
    setOpen(next.shown);
    if (opening) {
      markEndAtFrame("mailbox_open");
      requestAnimationFrame(() => {
        if (open() && key !== undefined && key.getClientRects().length === 0)
          column?.focus({ preventScroll: true });
      });
    }
    if (next.focus === "key" && focus !== "key") requestAnimationFrame(() => focusEdgeKey(keyId()));
  }
  createEffect(
    () => props.asked,
    (asked) => {
      if (asked === heard) return;
      heard = asked;
      feed("toggle");
    },
  );
  createEffect(wanted, (row) => {
    if (row === null) return;
    takeRow();
    const next = stepLetter(
      { mail: { shown: open(), focus: focusNow() }, opener: row, row: null },
      { kind: "close" },
    );
    setOpen(next.mail.shown);
    const target = next.row;
    if (target !== null)
      requestAnimationFrame(() => {
        column?.querySelector<HTMLElement>(`[data-letter="${CSS.escape(target)}"] button`)?.focus();
      });
  });
  function pressed(event: PointerEvent): void {
    const { target } = event;
    if (
      !(target instanceof Node) ||
      column?.contains(target) === true ||
      key?.contains(target) === true
    )
      return;
    feed("outside");
  }
  function escaped(event: KeyboardEvent): void {
    if (
      event.key !== "Escape" ||
      event.isComposing ||
      event.defaultPrevented ||
      column?.querySelector(":popover-open") != null
    )
      return;
    event.preventDefault();
    feed("escape");
  }
  createEffect(open, (shown) => {
    if (!shown) return;
    document.addEventListener("pointerdown", pressed, true);
    window.addEventListener("keydown", escaped);
    return () => {
      document.removeEventListener("pointerdown", pressed, true);
      window.removeEventListener("keydown", escaped);
    };
  });
  return (
    <>
      <span class="mailbox-key-host">
        <ControlTip id={hint} text={look().hint} side="right" exposable>
          <button
            ref={(node) => {
              key = node;
            }}
            id={keyId()}
            type="button"
            class="mailbox-key glass"
            data-control=""
            aria-label={look().key.wire["aria-label"] ?? undefined}
            aria-expanded={open() ? "true" : "false"}
            aria-describedby={hint}
            aria-controls={aside}
            onClick={() => {
              feed("toggle");
            }}
          >
            <Glyph name={look().glyph} size="key" />
            <span class="refrain-rail-caption" aria-hidden="true">
              {say(lang(), "edge_mailbox")}
            </span>
            <Show when={bar()}>
              {(kind) => (
                <span class="mailbox-key-foot" aria-hidden="true">
                  <i
                    class={["mailbox-key-bar", kind() === "connecting" && "pulse"]}
                    data-link={kind()}
                  />
                </span>
              )}
            </Show>
            <Show when={tally()}>
              {(count) => (
                <span class="mailbox-key-count" aria-hidden="true">
                  {count()}
                </span>
              )}
            </Show>
            <Show when={look().corner.kind === "fresh"}>
              <span class="mailbox-key-fresh" aria-hidden="true" />
            </Show>
          </button>
        </ControlTip>
      </span>
      <Show when={open()}>
        <aside
          ref={(node) => {
            column = node;
          }}
          id={aside}
          aria-label={say(lang(), "edge_mailbox")}
          tabindex={-1}
          class="push fixed inset-y-0 left-0 z-(--layer-mailbox) flex h-full w-[calc(440px+var(--spacing-margin)+var(--spacing-key)+var(--spacing-pane))] max-w-[100vw] flex-col overflow-hidden bg-raised pl-[calc(var(--spacing-margin)+var(--spacing-key)+var(--spacing-pane))] text-body text-text shadow-sheet narrow:w-full narrow:pl-0"
        >
          <MailboxColumn
            onClose={() => {
              feed("follow");
            }}
            onBack={() => {
              focusEdgeKey(keyId());
              feed("follow");
            }}
          />
        </aside>
      </Show>
    </>
  );
}

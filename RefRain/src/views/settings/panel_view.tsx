// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { Option } from "effect";
import { createEffect, createSignal, createUniqueId, onSettled, Show } from "solid-js";

import { say } from "../../core/lang";
import { fromFragment, type SetupGroup } from "../../core/route";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import Glyph from "../parts/glyph.solid";
import Setup from "../setup_view";
import SettingsTree, { type SettingsTreeProps } from "./tree_view";
import "./panel.css";

export interface SettingsPanelProps extends SettingsTreeProps {
  readonly onClose: () => void;
}

type Presentation = "both" | "index" | "detail";

const NARROW = "(width < 768px)";

export interface SettingsSheetProps extends SettingsTreeProps {
  readonly onBack: () => void;
  readonly presentation: Presentation;
  readonly titleId: string;
}

interface DialogProps {
  readonly titleId: string;
  readonly onClose: () => void;
  readonly visit?: (event: MouseEvent) => void;
  readonly hold?: (node: HTMLDialogElement) => void;
  readonly children?: JSX.Element;
}

export function PanelDialog(props: DialogProps) {
  return (
    <dialog
      ref={(node) => props.hold?.(node)}
      aria-labelledby={props.titleId}
      onCancel={(event) => {
        event.preventDefault();
        props.onClose();
      }}
      onClick={(event) => {
        if (event.target === event.currentTarget) props.onClose();
        else props.visit?.(event);
      }}
      class="settings-panel fixed inset-y-0 left-0 m-0 h-dvh max-h-none w-full max-w-[calc(var(--silver-side)*(1+var(--silver))-var(--spacing-gutter)/2)] bg-page narrow:max-w-none p-0 text-body text-text shadow-sheet backdrop:bg-transparent backdrop:backdrop-brightness-50"
    >
      {props.children}
    </dialog>
  );
}

export function SettingsSheet(props: SettingsSheetProps) {
  const lang = storeValue(ui().lang);
  const back = () =>
    say(lang(), props.presentation === "detail" ? "shell_go_back" : "settings_close");
  let body: HTMLElement | undefined;

  createEffect(
    () => props.group,
    () => {
      body?.scrollTo({ top: 0 });
    },
  );

  return (
    <div
      class="settings-sheet @container/sheet h-full min-h-0"
      data-settings-presentation={props.presentation}
    >
      <div class="settings-sheet-layout flex h-full min-h-0 @max-lg/sheet:flex-col">
        <div
          hidden={props.presentation === "detail"}
          inert={props.presentation === "detail"}
          class="settings-index flex w-[calc(var(--silver-side)-var(--spacing-gutter)/2)] shrink-0 flex-col gap-base overflow-y-auto border-r border-edge px-base py-wide @max-lg/sheet:w-full @max-lg/sheet:border-r-0 @max-lg/sheet:py-base"
        >
          <p
            id={`${props.titleId}-index`}
            tabindex="-1"
            class="px-snug text-label font-label text-text"
          >
            {say(lang(), "nav_settings")}
          </p>
          <SettingsTree
            group={props.group}
            beneath={props.beneath}
            onPick={(group) => props.onPick(group)}
          />
        </div>

        <section
          hidden={props.presentation === "index"}
          inert={props.presentation === "index"}
          ref={(node) => {
            body = node;
          }}
          class="settings-detail slide-then @container/page flex min-h-0 min-w-0 flex-1 flex-col overflow-y-auto"
          aria-labelledby={props.titleId}
        >
          <Show when={props.group} keyed>
            {(group) => (
              <div class="shift">
                <Setup group={group} titleId={props.titleId} />
              </div>
            )}
          </Show>
        </section>
      </div>
      <button
        type="button"
        data-control="surface"
        class="settings-back glass rounded-control text-note font-label text-text"
        aria-label={back()}
        onClick={() => props.onBack()}
      >
        <Glyph name="chevron" size="sm" class="shrink-0 rotate-180" />
        <span>{back()}</span>
      </button>
    </div>
  );
}

export default function SettingsPanel(props: SettingsPanelProps) {
  const titleId = `${createUniqueId()}-title`;
  const media = window.matchMedia(NARROW);
  const [narrow, setNarrow] = createSignal(media.matches);
  const opened = Option.getOrNull(fromFragment(window.location.hash));
  const [detail, setDetail] = createSignal(opened?.kind === "setup" && opened.group !== undefined);
  const [wanted, setWanted] = createSignal<SetupGroup>();
  const presentation = (): Presentation => (narrow() ? (detail() ? "detail" : "index") : "both");
  let sheet: HTMLDialogElement | undefined;

  function focusIndex(): void {
    if (sheet === undefined) return;
    const current = sheet.querySelector<HTMLElement>("nav [aria-current=true]");
    const label = sheet.ownerDocument.getElementById(`${titleId}-index`);
    (current ?? label)?.focus();
  }

  function focusDetail(): void {
    sheet?.ownerDocument.getElementById(titleId)?.focus();
  }

  onSettled(() => {
    if (sheet === undefined) return;
    sheet.showModal();
    if (presentation() === "detail") focusDetail();
    else focusIndex();
    function update(event: MediaQueryListEvent): void {
      setNarrow(event.matches);
    }
    media.addEventListener("change", update);
    return () => media.removeEventListener("change", update);
  });

  createEffect(
    () => wanted() === props.group && presentation() === "detail",
    (focus) => {
      if (!focus) return;
      setWanted(undefined);
      onSettled(focusDetail);
    },
  );

  function pick(group: SetupGroup): void {
    setDetail(true);
    if (narrow()) setWanted(group);
    props.onPick(group);
  }

  function back(): void {
    if (presentation() === "detail") {
      setDetail(false);
      setWanted(undefined);
      onSettled(focusIndex);
      return;
    }
    if (sheet?.open === true) sheet.close();
    props.onClose();
  }

  function visit(event: MouseEvent): void {
    if (
      event.defaultPrevented ||
      event.button !== 0 ||
      event.altKey ||
      event.ctrlKey ||
      event.metaKey ||
      event.shiftKey ||
      !(event.target instanceof Element)
    ) {
      return;
    }
    const link = event.target.closest<HTMLAnchorElement>("a[href]");
    const current = link?.ownerDocument.location;
    if (
      link === null ||
      current === undefined ||
      (link.target !== "" && link.target !== "_self") ||
      link.hasAttribute("download") ||
      link.origin !== current.origin ||
      link.pathname !== current.pathname ||
      link.search !== current.search
    ) {
      return;
    }
    const route = Option.getOrNull(fromFragment(link.hash));
    if (route?.kind !== "setup") return;
    event.preventDefault();
    pick(route.group ?? props.group);
  }

  return (
    <PanelDialog
      onClose={back}
      visit={visit}
      titleId={presentation() === "index" ? `${titleId}-index` : titleId}
      hold={(node) => {
        sheet = node;
      }}
    >
      <SettingsSheet
        group={props.group}
        beneath={props.beneath}
        onPick={pick}
        onBack={back}
        presentation={presentation()}
        titleId={titleId}
      />
    </PanelDialog>
  );
}

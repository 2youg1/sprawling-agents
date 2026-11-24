// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Show } from "solid-js";
import { POSTURE_WORD, type Doing } from "../../core/doing";
import { say, type Lang } from "../../core/lang";
import type { NavigationLayer } from "../../core/shell_visit";
import { CONVERSATION_ID, DensityKey, focusEdgeKey, NAVIGATION_KEY, scopedId } from "./chrome";
import type { ShellLayout } from "./layout";

export function toolActivity(doing: Doing): string {
  return doing.kind === "calling"
    ? [doing.tool, doing.subject].filter((value) => value !== null).join(" ")
    : "";
}

export function LayoutTools(props: {
  readonly lang: Lang;
  readonly scope: string | undefined;
  readonly root: () => HTMLElement | undefined;
  readonly shell: ShellLayout;
  readonly files: boolean;
  readonly panorama: boolean;
  readonly doing: Doing | undefined;
}) {
  const id = (part: string) => scopedId(part, props.scope);
  const returnToNav = () => focusEdgeKey(NAVIGATION_KEY, props.scope);
  function draft(): void {
    if (props.shell.draft && !props.shell.split) requestAnimationFrame(returnToNav);
    props.shell.closeNavigation(false);
    props.shell.toggleDraft();
  }

  function split(): void {
    const enabling = !props.shell.split;
    props.shell.closeNavigation(false);
    props.shell.toggleSplit();
    requestAnimationFrame(() => {
      const divider = enabling
        ? props
            .root()
            ?.querySelector<HTMLElement>(
              `[role="separator"][aria-controls="${id("workspace-file-canvas")}"]`,
            )
        : undefined;
      if (divider == null) returnToNav();
      else divider.focus({ preventScroll: true });
    });
  }

  return (
    <div class="refrain-nav-body refrain-layout-tools">
      <Show when={props.shell.peeking && !props.files}>
        <div
          class="refrain-activity-inline"
          role="status"
          aria-label={say(props.lang, "shell_active_run")}
        >
          <Show when={props.doing} fallback={say(props.lang, "shell_idle_run")}>
            {(doing) => (
              <p>
                {say(props.lang, POSTURE_WORD[doing().kind])} {toolActivity(doing())}
              </p>
            )}
          </Show>
        </div>
      </Show>
      <Show
        when={props.files}
        fallback={
          <DensityKey
            lang={props.lang}
            panorama={props.panorama}
            onToggle={props.shell.setDensity}
            onPeek={props.shell.setPeek}
          />
        }
      >
        <button
          id={id("workspace-draft-key")}
          class="refrain-nav-button"
          data-control="surface"
          type="button"
          aria-controls={id(CONVERSATION_ID)}
          aria-expanded={props.shell.draft || props.shell.split ? "true" : "false"}
          onClick={draft}
        >
          {say(props.lang, "shell_draft")}
        </button>
        <button
          class="refrain-nav-button"
          data-control="surface"
          type="button"
          aria-pressed={props.shell.split ? "true" : "false"}
          onClick={split}
        >
          {say(props.lang, "shell_split")}
        </button>
      </Show>
    </div>
  );
}

export function NavigatorFooter(props: {
  readonly lang: Lang;
  readonly files: boolean;
  readonly listed: boolean;
  readonly onNavigation: (layer: NavigationLayer, trigger?: HTMLElement) => void;
  readonly close: () => void;
}) {
  return (
    <footer>
      <div class="refrain-nav-links" hidden={!props.listed}>
        <button
          class="refrain-nav-button"
          data-control="surface"
          type="button"
          onClick={(event) => props.onNavigation("tools", event.currentTarget)}
        >
          {say(props.lang, "shell_layout_tools")}
        </button>
        <button
          class="refrain-nav-button"
          data-control="surface"
          type="button"
          onClick={(event) => props.onNavigation("history", event.currentTarget)}
        >
          {say(props.lang, "world_commits")}
        </button>
        <Show when={props.files}>
          <button
            class="refrain-nav-button"
            data-control="surface"
            type="button"
            onClick={(event) => props.onNavigation("library", event.currentTarget)}
          >
            {say(props.lang, "shell_library_tools")}
          </button>
        </Show>
      </div>
      <Show
        when={props.listed}
        fallback={
          <button
            class="refrain-nav-button"
            data-control="surface"
            type="button"
            onClick={() => props.onNavigation("list")}
          >
            {say(props.lang, "shell_go_back")}
          </button>
        }
      >
        <button
          class="refrain-nav-button"
          data-control="surface"
          type="button"
          onClick={props.close}
        >
          {say(props.lang, "shell_close_list")}
        </button>
      </Show>
    </footer>
  );
}

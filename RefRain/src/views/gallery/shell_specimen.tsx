// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createSignal, createUniqueId, Show } from "solid-js";
import { say } from "../../core/lang";
import type { Tier } from "../../core/prefs";
import type { ItemLink, SetupGroup } from "../../core/route";
import { memory } from "../../core/rows";
import {
  DEFAULT_SPLIT_LINE,
  DEFAULT_SPLIT_TOP,
  shellVisit,
  type Navigation,
  type Subject,
} from "../../core/shell_visit";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Address, Seq } from "../../wire";
import Mailbox from "../mailbox/mailbox";
import SettingsPanel from "../settings/panel_view";
import { Chrome, focusEdgeKey, MAILBOX_KEY, NAVIGATION_KEY, scopedId } from "../shell/chrome";
import type { ShellLayout } from "../shell/layout";
import { WorkspaceShell } from "../shell/workspace_shell";

interface Props {
  readonly address: Address;
  readonly session?: Seq;
  readonly tier: Tier;
  readonly height: number;
  readonly item?: ItemLink;
}

export default function ShellSpecimen(props: Props) {
  const scope = createUniqueId();
  const lang = storeValue(ui().lang);
  const [subject, setSubject] = createSignal<Subject>("session");
  const [tier, setTier] = createSignal<Tier>(props.tier === "panorama" ? "panorama" : "zen");
  const [navigation, setNavigation] = createSignal<Navigation>("closed");
  const [draft, setDraft] = createSignal(false);
  const [split, setSplit] = createSignal(false);
  const [splitLine, resizeSplit] = createSignal(DEFAULT_SPLIT_LINE);
  const [splitTop, resizeSplitTop] = createSignal(DEFAULT_SPLIT_TOP);
  const [peeked, setPeek] = createSignal(props.tier === "blend");
  const [settings, setSettings] = createSignal<SetupGroup>();
  const visit = shellVisit(memory());
  let settingsKey: HTMLElement | undefined;

  const shell: ShellLayout = {
    get subject() {
      return subject();
    },
    get navigation() {
      return navigation();
    },
    get draft() {
      return draft();
    },
    get split() {
      return split();
    },
    get splitLine() {
      return splitLine();
    },
    get splitTop() {
      return splitTop();
    },
    get peeking() {
      return peeked();
    },
    get navigationCovered() {
      return settings() !== undefined;
    },
    visit,
    resizeSplit,
    resizeSplitTop,
    closeNavigation(restore = true) {
      setNavigation("closed");
      if (restore) requestAnimationFrame(() => focusEdgeKey(NAVIGATION_KEY, scope));
    },
    selectNavigation: setNavigation,
    setDensity: () => {
      setTier((shown) => (shown === "zen" ? "panorama" : "zen"));
    },
    setPeek,
    toggleDraft: () => {
      setDraft((shown) => !shown);
    },
    toggleSplit: () => {
      setSplit((shown) => !shown);
    },
    showDraft: () => {
      setDraft(true);
    },
  };

  const closeSettings = () => {
    setSettings(undefined);
    settingsKey?.focus({ preventScroll: true });
  };

  return (
    <div class="@container/shell">
      <div
        class="frame relative translate-x-0 overflow-hidden bg-page"
        style={{ height: `${props.height}px` }}
        data-refrain-session-shell={subject() === "session" ? "" : undefined}
      >
        <WorkspaceShell
          scope={scope}
          address={props.address}
          session={props.session}
          tier={tier()}
          item={props.item}
          shell={shell}
        />
        <Show when={settings()}>
          {(group) => (
            <SettingsPanel
              group={group()}
              beneath={{ kind: "talk", address: props.address }}
              onPick={setSettings}
              onClose={closeSettings}
            />
          )}
        </Show>
        <nav
          class="edge-keys refrain-edge"
          data-refrain-edge=""
          aria-label={say(lang(), "region_edge")}
        >
          <Chrome
            scope={scope}
            lang={lang()}
            subject={subject()}
            navigation={navigation() !== "closed"}
            settingsOpen={settings() !== undefined}
            mailbox={
              <Mailbox asked={0} keyId={scopedId(MAILBOX_KEY, scope)} hint={(text) => text} />
            }
            onNavigation={() => {
              if (navigation() === "closed") setNavigation("list");
              else shell.closeNavigation();
            }}
            onSubject={() => {
              setSubject((shown) => (shown === "files" ? "session" : "files"));
              setNavigation("closed");
            }}
            onSettings={(trigger) => {
              settingsKey = trigger;
              setSettings("appearance");
            }}
          />
        </nav>
      </div>
    </div>
  );
}

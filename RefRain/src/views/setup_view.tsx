// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { createEffect, createMemo, createSignal, Match, Show, Switch } from "solid-js";

import { QUERIES } from "../core/asking";
import { setAutonomy } from "../core/commands";
import { endonym, LANGS, say, type Key, type Lang } from "../core/lang";
import type { SetupGroup } from "../core/route";
import { storeValue } from "../core/solid_store";
import { ui } from "../ui";
import { ResidentId, type Autonomy, type EndpointsAnswer, type Proxying } from "../wire";
import Machine from "./machine.solid";
import Glyph from "./parts/glyph.solid";
import Segmented from "./parts/segmented.solid";
import Release from "./release_view";
import Automation from "./settings/automation_view";
import CityLayer from "./settings/city_layer_view";
import ContextRung from "./settings/context_rung_view";
import Devices from "./settings/devices_view";
import Performance from "./settings/performance_view";
import Remote from "./settings/remote_view";
import Rules from "./settings/rules_view";
import You from "./settings/you_view";
import WritingView from "./settings/writing_view";
import AdvancedSection from "./setup/advanced_view";
import { saveReceipt } from "./setup/appearance";
import AppearanceSection from "./setup/appearance_view";
import ColoursSection from "./setup/colours_view";
import Decided from "./setup/decided_view";
import GovernedSection from "./setup/governed_view";
import { HEADING, HINT, PREFERRED, WIDTH } from "./setup/groups";
import Kept from "./setup/kept_view";
import KeysSection from "./setup/keys_view";
import ModelChoice from "./setup/models_view";
import PrivacySection from "./setup/privacy/privacy_view";
import { PROXYINGS, proxyingNote } from "./setup/providers/draft";
import SearchCard from "./setup/search/search_view";
import SkillsSection from "./setup/skills_view";
import Toml from "./setup/toml_view";
import Agents from "./setup/agents_view";
import EffortSection from "./shared/effort_view";
import ProviderDoor from "./shared/provider.solid";

type Setting = "proxying" | "autonomy" | "language";
type AutonomySetting = "owner" | "delegate";

const AUTONOMIES = [
  ["owner", "autonomy_owner"],
  ["delegate", "autonomy_clerk"],
] as const satisfies readonly (readonly [AutonomySetting, Key])[];

function autonomySetting(held: Autonomy): AutonomySetting {
  return typeof held === "string" ? held : "delegate";
}

function autonomyOf(setting: AutonomySetting): Autonomy {
  return setting === "owner" ? "owner" : { delegate: ResidentId.make("hall/clerk") };
}

export interface SetupProps {
  readonly group?: SetupGroup | undefined;
  readonly titleId?: string | undefined;
  readonly endpoints?: EndpointsAnswer | undefined;
  readonly autonomy?: Autonomy | undefined;
  readonly onAutonomy?: ((setting: AutonomySetting) => void) | undefined;
  readonly dependency?: (() => JSX.Element) | undefined;
}

export default function Setup(props: SetupProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const held = storeValue(u.prefs.held);
  const keeper = storeValue(u.prefs.keeper);
  const endpoints = storeValue(u.conn.asking.ask(QUERIES.endpoints));
  const governance = storeValue(u.conn.asking.ask(QUERIES.governance));
  const shown = () => props.group ?? "you";
  const word = (key: Key) => say(lang(), key);
  const answer = createMemo(() => {
    const stated = endpoints();
    return (
      props.endpoints ??
      (stated !== undefined && "endpoints" in stated ? stated.endpoints : undefined)
    );
  });
  const chosen = createMemo(() => {
    const stated = governance();
    return (
      props.autonomy ??
      (stated !== undefined && "governance" in stated ? stated.governance.autonomy : undefined)
    );
  });
  const hint = () => HINT[shown()];
  const note = () => proxyingNote(held().proxying);

  const receipt = saveReceipt();
  const saved = storeValue(receipt.saved);
  const [awaited, setAwaited] = createSignal<AutonomySetting>();
  createEffect(
    () => ({ sent: awaited(), received: chosen() }),
    ({ sent, received }) => {
      if (sent === undefined || received === undefined || autonomySetting(received) !== sent)
        return;
      setAwaited(undefined);
      receipt.landed("autonomy");
    },
  );

  function settle(next: AutonomySetting): void {
    setAwaited(next);
    if (props.onAutonomy !== undefined) {
      props.onAutonomy(next);
      return;
    }
    u.send(setAutonomy("city", autonomyOf(next)));
  }

  function Foot(props: { readonly name: Setting; readonly constraint?: Key | undefined }) {
    return (
      <div class="mt-tight flex items-baseline justify-between gap-base">
        <Show when={props.constraint} keyed>
          {(constraint) => <span class="text-note text-text-faint">{word(constraint)}</span>}
        </Show>
        <Show when={saved() === props.name}>
          <span class="fade ml-auto inline-flex items-center gap-tight text-note text-text-quiet">
            <Glyph name="check" size="sm" class="shrink-0" />
            {word("setup_saved")}
          </span>
        </Show>
      </div>
    );
  }

  function LanguageCard() {
    return (
      <div class="flex flex-col gap-tight rounded-card bg-raised px-base py-snug">
        <span class="text-label font-label text-text">{word("setup_language")}</span>
        <p class="text-note text-text-faint">{word("setup_language_note")}</p>
        <Segmented
          label={word("setup_language")}
          options={LANGS.map((each) => ({ value: each, label: endonym(each) }))}
          held={held().lang}
          onPick={(next: Lang) => {
            u.prefs.setLang(next);
            receipt.landed("language");
          }}
        />
        <Foot name="language" />
      </div>
    );
  }

  function RunCards() {
    return (
      <div class="grid grid-fit items-start gap-base">
        <GovernedSection />
        <div class="flex flex-col gap-tight rounded-card bg-raised px-base py-snug">
          <span class="text-label font-label text-text">{word("setup_autonomy")}</span>
          <p class="text-note text-text-faint">{word("setup_autonomy_note")}</p>
          <Show when={chosen()}>
            {(current) => (
              <Segmented
                label={word("setup_autonomy")}
                options={AUTONOMIES.map(([setting, key]) => ({
                  value: setting,
                  label: word(key),
                }))}
                held={autonomySetting(current())}
                onPick={settle}
              />
            )}
          </Show>
          <Foot name="autonomy" />
          <Decided />
        </div>
        <ContextRung />
      </div>
    );
  }

  function NetworkCard() {
    return (
      <div class="flex flex-col gap-tight rounded-card bg-raised px-base py-snug">
        <span class="text-label font-label text-text">{word("setup_network_default")}</span>
        <p class="text-note text-text-faint">{word("setup_network_new_only")}</p>
        <Segmented
          label={word("setup_network_default")}
          options={PROXYINGS.map(([setting, key]) => ({ value: setting, label: word(key) }))}
          held={held().proxying}
          onPick={(next: Proxying) => {
            u.prefs.setProxying(next);
            receipt.landed("proxying");
          }}
        />
        <Foot name="proxying" constraint={note()} />
      </div>
    );
  }

  function AccountsCards() {
    return (
      <div class="flex flex-col gap-wide">
        <div class="min-w-0">
          <ProviderDoor stepped />
        </div>
        <section class="flex min-w-0 flex-col gap-snug border-t border-edge pt-wide">
          <header class="flex min-w-0 flex-col gap-tight">
            <h3 class="text-label font-label text-text">{word("setup_step_models")}</h3>
            <p class="text-note text-text-faint">{word("setup_step_models_hint")}</p>
          </header>
          <Show when={answer()}>{(current) => <ModelChoice answer={current()} />}</Show>
        </section>

        <section class="flex min-w-0 flex-col gap-snug border-t border-edge pt-wide">
          <header class="flex min-w-0 flex-col gap-tight">
            <h3 class="text-label font-label text-text">{word("setup_step_defaults")}</h3>
            <p class="text-note text-text-faint">{word("setup_step_defaults_hint")}</p>
          </header>
          <div class="grid grid-fit items-start gap-base">
            <div class="min-w-0 rounded-card bg-raised px-base py-snug">
              <EffortSection />
            </div>
            <CityLayer />
          </div>
        </section>
        <section class="flex min-w-0 max-w-talk flex-col gap-snug border-t border-edge pt-wide">
          <header class="flex min-w-0 flex-col gap-tight">
            <h3 class="text-label font-label text-text">{word("setup_step_search")}</h3>
            <p class="text-note text-text-faint">{word("setup_step_search_hint")}</p>
          </header>
          <SearchCard />
        </section>
      </div>
    );
  }

  return (
    <div class="flex min-w-0 flex-col gap-wide px-wide py-wide narrow:px-base">
      <header class="flex min-w-0 flex-col gap-tight border-b border-edge pb-base">
        <div class="flex flex-wrap items-center gap-base">
          <h2 id={props.titleId} class="text-title font-title" tabindex="-1">
            {word(HEADING[shown()])}
          </h2>
          <Show when={PREFERRED.includes(shown())}>
            <Kept keeper={keeper()} />
          </Show>
        </div>
        <Show when={hint()} keyed>
          {(note) => <p class="text-note text-text-quiet">{word(note)}</p>}
        </Show>
      </header>
      <div class={["flex min-w-0 flex-col gap-base", WIDTH[shown()]]}>
        <Switch>
          <Match when={shown() === "you"}>
            <You />
          </Match>
          <Match when={shown() === "accounts"}>
            <AccountsCards />
          </Match>
          <Match when={shown() === "agents"}>
            <Agents />
          </Match>
          <Match when={shown() === "devices"}>
            <Devices />
          </Match>
          <Match when={shown() === "run"}>
            <RunCards />
          </Match>
          <Match when={shown() === "rules"}>
            <Rules />
          </Match>
          <Match when={shown() === "performance"}>
            <Performance />
          </Match>
          <Match when={shown() === "automation"}>
            <Automation />
          </Match>
          <Match when={shown() === "writing"}>
            <WritingView lang={lang()} />
          </Match>
          <Match when={shown() === "network"}>
            <NetworkCard />
          </Match>
          <Match when={shown() === "remote"}>
            <Remote />
          </Match>
          <Match when={shown() === "tools"}>
            <Show when={props.dependency} keyed fallback={<Machine />}>
              {(fixture) => fixture()}
            </Show>
          </Match>
          <Match when={shown() === "skills"}>
            <SkillsSection />
          </Match>
          <Match when={shown() === "appearance"}>
            <div class="grid grid-fit items-start gap-base">
              <AppearanceSection />
              <LanguageCard />
            </div>
          </Match>
          <Match when={shown() === "colours"}>
            <ColoursSection />
          </Match>
          <Match when={shown() === "keys"}>
            <KeysSection />
          </Match>
          <Match when={shown() === "advanced"}>
            <AdvancedSection />
          </Match>
          <Match when={shown() === "privacy"}>
            <PrivacySection />
          </Match>
          <Match when={shown() === "about"}>
            <Release />
          </Match>
        </Switch>
      </div>
      <Toml />
    </div>
  );
}

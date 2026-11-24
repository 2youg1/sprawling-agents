// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, Match, Show, Switch } from "solid-js";
import { QUERIES } from "../../core/asking";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { GuideStep } from "../../wire";
import Machine from "../machine.solid";
import GovernedSection from "../setup/governed_view";
import ModelChoice from "../setup/models_view";
import SkillsSection from "../setup/skills_view";
import ProviderDoor from "../shared/provider.solid";
import { mcpDoorOf } from "./door";
import Door from "./door.look.solid";

function ProviderStep() {
  const u = ui();
  const endpoints = storeValue(u.conn.asking.ask(QUERIES.endpoints));
  const answer = createMemo(() => {
    const held = endpoints();
    return held !== undefined && "endpoints" in held ? held.endpoints : undefined;
  });
  const mainMissing = () => {
    const held = answer();
    return held !== undefined && !held.chosen.some((chosen) => chosen.tag === "main");
  };
  const models = () => {
    const held = answer();
    return held?.endpoints.length ? held : undefined;
  };
  return (
    <div class="flex min-w-0 max-w-talk flex-col gap-wide">
      <ProviderDoor openModels={mainMissing()} />
      <Show when={models()}>{(answer) => <ModelChoice answer={answer()} tags={["main"]} />}</Show>
    </div>
  );
}

export default function WelcomeBody(props: { readonly step: GuideStep }) {
  const lang = storeValue(ui().lang);
  return (
    <Switch>
      <Match when={props.step === "provider"}>
        <ProviderStep />
      </Match>
      <Match when={props.step === "dependencies"}>
        <Machine />
      </Match>
      <Match when={props.step === "texts"}>
        <GovernedSection />
      </Match>
      <Match when={props.step === "skills"}>
        <SkillsSection />
      </Match>
      <Match when={props.step === "mcp"}>
        <Door {...mcpDoorOf(lang())} />
      </Match>
    </Switch>
  );
}

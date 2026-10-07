<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The conversation's small looks and the welcome guide's two, each
  // drawn from a value of its own type with the states its seat hands
  // it, so the roles and boxes each look draws are read on this page.
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import AsideLink from "../talk/aside_link.look.svelte";
  import Checkpoint from "../talk/checkpoint.look.svelte";
  import Fact from "../talk/fact.look.svelte";
  import Fold from "../talk/fold.look.svelte";
  import { foldWire } from "../talk/fold";
  import ForkButton from "../talk/fork_button.look.svelte";
  import type { ForkWire } from "../talk/fork_button";
  import Inbox from "../talk/inbox.look.svelte";
  import TypedLine from "../talk/typed_line.look.svelte";
  import Door from "../welcome/door.look.svelte";
  import { mcpDoorOf } from "../welcome/door";
  import Step from "../welcome/step.look.svelte";
  import Case from "./case.svelte";

  const { lang } = ui();
  const still = (): void => undefined;
  // Fixture data: what a city would have reported.
  const WORDS = { files: "3 files", asks: "waiting on you" };
  const fork: ForkWire = {
    type: "button",
    onmouseenter: still,
    onmouseleave: still,
    onfocus: still,
    onblur: still,
    onclick: still,
  };
</script>

<Case label="thread facts · a plain fact, one that asks, a fold, a fork key and an aside link">
  <div class="flex flex-wrap items-center gap-base">
    <Fact glyph="check" heard="done">{WORDS.files}</Fact>
    <Fact asks={true}>{WORDS.asks}</Fact>
    <Fold label="2 more" wire={foldWire(false, still)} />
    <ForkButton label="branch" hint="branch from this turn" wire={fork} />
    <AsideLink text="open the run" href="#/run/7f3a" />
  </div>
</Case>

<Case label="thread inbox · two letters waiting in this room">
  <Inbox
    label="inbox"
    heading="2 waiting"
    rows={[
      { key: "1", href: "#/talk/shop/checkout", kind: "letter", first: "The total is fixed.", from: "checkout", ago: "2m" },
      { key: "2", href: undefined, kind: "handback", first: "Tests pass on main.", from: "ci", ago: "5m" },
    ]}
  />
</Case>

<Case label="composer marks · the typed line lit and resting, two checkpoints on a line">
  <div class="flex flex-col gap-base">
    <div class="relative h-control">
      <TypedLine typed={120} lit={true} />
    </div>
    <div class="relative h-control">
      <TypedLine typed={0} lit={false} />
    </div>
    <div class="relative h-control">
      <Checkpoint reminder="first" at={60} along="line" />
      <Checkpoint reminder="second" at={85} along="line" />
    </div>
  </div>
</Case>

<Case label="welcome steps · one configured, one required with its body open, and the MCP door">
  <div class="flex flex-col">
    <Step
      number="1"
      title={say($lang, "guide_step_provider")}
      standing="configured"
      word={say($lang, "guide_standing_configured")}
      wire={{
        id: "fixture-step-1",
        type: "button",
        "aria-expanded": false,
        "aria-controls": "fixture-step-1-body",
        onclick: still,
      }}
    />
    <Step
      number="5"
      title={say($lang, "guide_step_mcp")}
      standing="required"
      word={say($lang, "guide_standing_required")}
      wire={{
        id: "fixture-step-5",
        type: "button",
        "aria-expanded": true,
        "aria-controls": "fixture-step-5-body",
        onclick: still,
      }}
    />
    <div id="fixture-step-5-body">
      <Door {...mcpDoorOf($lang)} />
    </div>
  </div>
</Case>

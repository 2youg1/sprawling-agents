<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The building page's looks, each drawn from a value of its own type
  // with the states its seat hands it, so the roles and boxes each look
  // draws are read here rather than only inside a whole building page.
  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import ArmSelect from "../building/arm_select.look.svelte";
  import { armSelectWire } from "../building/arm_select";
  import Disclosure from "../building/disclosure.look.svelte";
  import { disclosureWire } from "../building/disclosure";
  import FactLink from "../building/fact_link.look.svelte";
  import FileRow from "../building/file_row.look.svelte";
  import { fileRowWire } from "../building/file_row";
  import RoomRow from "../building/room_row.look.svelte";
  import RunRow from "../building/run_row.look.svelte";
  import Sections from "../building/sections.look.svelte";
  import { sectionsLookOf } from "../building/sections";
  import SkillRow from "../building/skill_row.look.svelte";
  import SourceToggle from "../building/source_toggle.look.svelte";
  import { sourceToggleWire } from "../building/source_toggle";
  import TalkLink from "../building/talk_link.look.svelte";
  import TextBox from "../building/text_box.look.svelte";
  import { textBoxWire } from "../building/text_box";
  import TreeRow from "../building/tree_row.look.svelte";
  import { treeRowWire } from "../building/tree_row";
  import type { TreeMark } from "../building/tree_row";
  import Prompts from "../building/prompts.look.svelte";
  import { actionName, actionWord, cardsOf } from "../building/prompts";
  import type { PromptsLook } from "../building/prompts";
  import { Address, B3Hash, RunId } from "../../wire";
  import type { CatalogAnswer, LayerDocument } from "../../wire";
  import Catalog from "../building/catalog.look.svelte";
  import { catalogLook, draftOf, pressGroup, rowsOf } from "../building/catalog";
  import Case from "./case.svelte";

  const { lang } = ui();
  const still = (): void => undefined;
  const SHOP = Address.make("shop");
  const VERSION = B3Hash.make("af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262");
  // One room's prompt documents: the city's City.md replaced by the
  // building's, a project file withheld, an identity absent, and a note
  // the building has no place of its own for yet.
  const LAYERS: readonly LayerDocument[] = [
    { at: Address.make("City.md"), version: VERSION, slot: "city", kind: "city", layer: "city", state: { replaced: { by: Address.make("shop/.sprawling/City.md") } }, shipped: { version: VERSION, standing: "edited" } },
    { at: Address.make("shop/.sprawling/City.md"), version: VERSION, slot: "city", kind: "city", layer: "building", state: { sent: { bytes: 2048 } }, shipped: null },
    { at: Address.make("shop/AGENTS.md"), version: VERSION, slot: "building", kind: "conventions", layer: "building", state: { withheld: { why: "AGENTS.md is a directory, not a file" } }, shipped: null },
    { at: Address.make("shop/checkout/URBANITE.md"), version: VERSION, slot: "resident", kind: "identity", layer: "room", state: "absent", shipped: null },
    { at: Address.make(".sprawling/models/local/m-1.md"), version: VERSION, slot: "run", kind: { model_note: { provider: "local", model: "m-1" } }, layer: "city", state: { sent: { bytes: 96 } }, shipped: null },
    { at: Address.make("shop/.sprawling/models/local/m-1.md"), version: VERSION, slot: "run", kind: { model_note: { provider: "local", model: "m-1" } }, layer: "building", state: "absent", shipped: null },
  ];
  const PROMPTS = $derived.by((): PromptsLook => ({
    rooms: [
      { label: "shop", current: false, onPress: still },
      { label: "shop/checkout", current: true, onPress: still },
    ],
    roomsLabel: say($lang, "prompts_rooms"),
    heading: fill(say($lang, "prompts_last_run"), { room: "shop/checkout" }),
    slots: [
      { key: "city", slot: say($lang, "slot_city"), size: "2,048" },
      { key: "building", slot: say($lang, "slot_building"), size: "1,310" },
      { key: "resident", slot: say($lang, "slot_resident"), size: "4,120" },
      { key: "run", slot: say($lang, "slot_run"), size: "512" },
    ],
    run: { label: say($lang, "prompts_open_run"), href: "#/run/00000000-0000-4000-8000-000000000001/prompt" },
    note: say($lang, "prompts_next_run"),
    documentsLabel: say($lang, "prompts_documents"),
    cards: cardsOf(LAYERS, SHOP, $lang),
    actionLabel: (action) => actionWord(action, $lang),
    actionName: (card, action) => actionName(card, action, $lang),
    onAction: still,
  }));
  // One building's tools after its newest run: the city's own namespace
  // with a fixed row and an edited one, and a server the building turned
  // off; the draft moves `plan` into the core and the server back on, so
  // the meter shows what the save would add.
  const ANSWER: CatalogAnswer = {
    building: SHOP,
    run: null,
    core_bytes: 2840,
    index_bytes: 412,
    runs_seen: 12,
    tools: [
      { name: "edit", namespace: "sprawling", hint: "Replace text in a file you have read", bytes: 980, exposure: "core", from: "default", fallback: "core", runs_used: 11, last_used: null },
      { name: "plan", namespace: "sprawling", hint: "Claim a node of the roadmap", bytes: 640, exposure: "dormant", from: "default", fallback: "dormant", runs_used: 0, last_used: null },
      { name: "read", namespace: "sprawling", hint: "Open one file by its path", bytes: 860, exposure: "core", from: "fixed", fallback: "core", runs_used: 12, last_used: null },
      { name: "github_create_issue", namespace: "github", hint: "create a issue on the forge", bytes: 310, exposure: "off", from: "building", fallback: "dormant", runs_used: 0, last_used: null },
      { name: "github_list_issue", namespace: "github", hint: "list a issue on the forge", bytes: 290, exposure: "off", from: "building", fallback: "dormant", runs_used: 0, last_used: null },
    ],
    city: {},
    stated: { namespaces: { github: "off" } },
  };
  const ROWS = rowsOf(ANSWER, "building");
  const DRAFT = pressGroup(ROWS, { tools: { plan: "core" }, namespaces: draftOf(ANSWER, "building").namespaces }, "github", "dormant");
  const CATALOG = $derived(
    catalogLook({ answer: { ...ANSWER, run: RunId.make("00000000-0000-4000-8000-000000000001") }, scope: "building", draft: DRAFT, edited: true, now: 0 }, $lang, {
      scope: still,
      edit: still,
      save: still,
    }),
  );
  const EMPTY = $derived(catalogLook({ answer: ANSWER, scope: "city", draft: draftOf(ANSWER, "city"), edited: false, now: 0 }, $lang, { scope: still, edit: still, save: still }));
  const TASK = "fix the checkout total";

  const MARKS: readonly { readonly name: string; readonly mark: TreeMark; readonly size: string | undefined }[] = [
    { name: "crates", mark: { kind: "directory", open: true }, size: undefined },
    { name: "src", mark: { kind: "directory", open: false }, size: undefined },
    { name: "Cargo.toml", mark: { kind: "file" }, size: "1.2 KB" },
    { name: "fix the checkout total", mark: { kind: "transcript", hint: "transcript" }, size: undefined },
  ];
</script>

<Case label="building tree row · a directory open, one shut, a file and a transcript">
  <ul class="flex flex-col">
    {#each MARKS as row, at (row.name)}
      <li>
        <TreeRow
          name={row.name}
          mark={row.mark}
          tone={at === 2 ? "picked" : "plain"}
          coded={false}
          live={at === 0 ? "1 run" : undefined}
          size={row.size}
          wire={treeRowWire(row.mark, still)}
        />
      </li>
    {/each}
  </ul>
</Case>

<Case label="building rows · a room, a run, a changed file and a skill">
  <div class="flex flex-col gap-tight">
    <RoomRow name="checkout" live="2 runs" wire={{ type: "button", onclick: still }} />
    <RunRow
      task="fix the checkout total"
      live={true}
      posture="working"
      spent="$0.42"
      started="10:24"
      wire={{ href: "#/run/7f3a" }}
    />
    <FileRow
      how="M"
      path="shop/checkout/total.rs"
      lines="+12 −3"
      current={true}
      wire={fileRowWire("shop/checkout/total.rs", true, still)}
    />
    <SkillRow
      name="release-notes"
      summary="Write the release notes from the merged pull requests."
      shelf="repo"
      at="#/skills/release-notes"
      admitted="admitted"
      used="3 uses"
      changed={say($lang, "skills_changed")}
      wire={{ type: "button", disabled: false, onclick: still }}
    />
  </div>
</Case>

<Case label="building sections · the index with the middle column's section current">
  <Sections
    {...sectionsLookOf(
      [
        { key: "plan", label: say($lang, "bld_plan") },
        { key: "commits", label: say($lang, "bld_commits") },
        { key: "catalog", label: say($lang, "bld_catalog") },
      ],
      "commits",
      still,
    )}
  />
</Case>

<Case label="building facts · a link, a figure, a talk link and a disclosure row">
  <div class="flex flex-col gap-tight">
    <div class="flex items-center gap-base">
      <FactLink label="1b2c3d4" face="figure" ink="text" wire={{ href: "#/building/shop" }} />
      <FactLink
        label="checkout"
        face="words"
        ink="quiet"
        wire={{ type: "button", "aria-expanded": false, onclick: still }}
      />
      <TalkLink label="checkout" wire={{ href: "#/talk/shop/checkout" }} />
    </div>
    <Disclosure layout="commit" open={false} wire={disclosureWire(false, still)} cells={row} />
  </div>
</Case>

{#snippet row()}
  <span class="font-mono">1b2c3d4</span>
  <span>{TASK}</span>
{/snippet}

<Case label="building controls · an arm choice, a source key and two text boxes">
  <div class="flex flex-col gap-tight">
    <div class="flex items-center gap-base">
      <ArmSelect
        options={[
          { value: "a", label: "arm a" },
          { value: "b", label: "arm b" },
        ]}
        wire={armSelectWire("arm", "a", still)}
      />
      <SourceToggle label="Markdown" pressed={true} wire={sourceToggleWire(true, still)} />
    </div>
    <TextBox size="list" wire={textBoxWire("shop/**\ndocs/**", "paths", false, still)} />
    <TextBox size="document" wire={textBoxWire("# Plan\n\nFix the total.", "plan", true, still)} />
  </div>
</Case>

<Case label="building prompts · the newest run's slots and one room's documents, each with its steps">
  <Prompts {...PROMPTS} />
</Case>

<Case label="building capabilities · a fixed row, a draft that moves one tool in and a server back on, and the meter">
  <Catalog {...CATALOG} />
</Case>

<Case label="building capabilities · the city's layer before the building's first run">
  <Catalog {...EMPTY} />
</Case>

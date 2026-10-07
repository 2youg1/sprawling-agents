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
  import { say } from "../../core/lang";
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
  import Case from "./case.svelte";

  const { lang } = ui();
  const still = (): void => undefined;
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
        { key: "skills", label: say($lang, "bld_skills") },
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

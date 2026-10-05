<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts" module>
  // The doctor page's three readings in every state they take: a
  // harness card in each of its three states, the release answer with
  // both registries and the update command for each channel, and the
  // scan in front of the city's directory read on Windows, stopped, and
  // on macOS or Linux where it does not apply.

  import type { AxError, DoctorScanning, HarnessLine, ReleaseAnswer } from "../../wire";

  const HARNESSES: readonly HarnessLine[] = [
    {
      name: "claude_code",
      docs: "https://docs.anthropic.com/en/docs/claude-code",
      launch: ["npx.cmd", "-y", "@anthropic-ai/claude-code"],
      state: { not_set_up: { looked: ["~/.claude", "%APPDATA%/claude"] } },
    },
    {
      name: "codex",
      docs: "https://developers.openai.com/codex",
      launch: ["npx.cmd", "-y", "@openai/codex"],
      state: { ready: { at: "~/.codex" } },
    },
    {
      name: "kimi_code",
      docs: "https://www.kimi.com/coding",
      launch: ["uvx", "kimi-cli"],
      state: { launcher_missing: { program: "uvx" } },
    },
  ];

  const MINE = { version: "0.0.8", released: "2026-09-01" };
  const NEWEST = { version: "0.0.9", released: "2026-10-02" };

  const REFUSAL: AxError = {
    action: "read the newest release",
    code: "E_TIMEOUT",
    nearby: [],
    recovery: "check that this computer reaches registry.npmjs.org, then press check again",
    retry: "yes",
    subject: "registry.npmjs.org",
  };

  const RELEASES: readonly (readonly [string, ReleaseAnswer])[] = [
    [
      "behind · installed through npm",
      {
        stands: {
          mine: MINE,
          newest: NEWEST,
          verdict: "behind",
          registries: [
            { registry: "npm", reading: { read: { newest: NEWEST } } },
            { registry: "crates_io", reading: { read: { newest: NEWEST } } },
          ],
          update: { alternatives: [], channel: "npm", command: "npm install -g sprawling@latest" },
        },
      },
    ],
    [
      "behind · Cargo operand differs from npm",
      {
        stands: {
          mine: MINE,
          newest: { version: "1.0.0", released: "" },
          verdict: "behind",
          registries: [
            { registry: "npm", reading: { read: { newest: NEWEST } } },
            { registry: "crates_io", reading: { read: { newest: { version: "1.0.0", released: "" } } } },
          ],
          update: { alternatives: ["cargo install sprawling --locked", "cargo binstall sprawling --locked"], channel: "cargo_or_binstall", command: null },
        },
      },
    ],
    [
      "current · installed through cargo",
      {
        stands: {
          mine: NEWEST,
          newest: NEWEST,
          verdict: "current",
          registries: [
            { registry: "npm", reading: { read: { newest: NEWEST } } },
            { registry: "crates_io", reading: { read: { newest: NEWEST } } },
          ],
          update: { alternatives: [], channel: "cargo", command: "cargo install sprawling --locked" },
        },
      },
    ],
    [
      "built from source · npm refused",
      {
        unreleased: {
          registries: [
            { registry: "npm", reading: { refused: { refusal: REFUSAL } } },
            { registry: "crates_io", reading: { refused: { refusal: REFUSAL } } },
          ],
          update: { alternatives: [], channel: "source" },
        },
      },
    ],
  ];

  const SCANS: readonly (readonly [string, DoctorScanning])[] = [
    [
      "Windows · a Dev Drive, outside every exclusion",
      { read: { city: "D:/city", drive: "trusted", exclusion: "outside" } },
    ],
    [
      "Windows · not a Dev Drive, inside an exclusion",
      { read: { city: "C:/city", drive: { not: { volume: "C:", file_system: "NTFS" } }, exclusion: { inside: { under: "C:/city" } } } },
    ],
    [
      "Windows · neither could be read",
      {
        read: {
          city: "C:/city",
          drive: { untold: { why: "admin_only" } },
          exclusion: { untold: { why: { failed: { command: "Get-MpPreference", code: 1 } } } },
        },
      },
    ],
    ["Windows · real-time scanning off", "stopped"],
    ["macOS and Linux · does not apply", "does_not_apply"],
  ];

  // The copy key beside the two kinds of text it carries: a command a
  // User runs, and an object name too long to read aloud. Its note
  // appears on hover and on keyboard focus; the resting face is the
  // icon and the name.
  const COPIED: readonly string[] = ["sprawling doctor --install", "9f2c4e1a7b3d5f60812e4a9c0b7d6e5f43a2b1c0"];

  const WIDTHS = [1040, 390] as const;
</script>

<script lang="ts">
  import Copy from "../machine/copy.svelte";
  import Scanning from "../machine/scanning.svelte";
  import ReleaseAnswerView from "../release/answer.svelte";
  import HarnessCards from "../setup/harness_cards.svelte";
  import Case from "./case.svelte";
</script>

{#each WIDTHS as width (width)}
  <Case label={`doctor · harnesses, three states at ${String(width)}`} {width}>
    <HarnessCards lines={HARNESSES} />
  </Case>
{/each}
{#each RELEASES as [name, answer] (name)}
  {#each WIDTHS as width (width)}
    <Case label={`doctor · release, ${name} at ${String(width)}`} {width}>
      <ReleaseAnswerView {answer} />
    </Case>
  {/each}
{/each}
{#each SCANS as [name, scanning] (name)}
  {#each WIDTHS as width (width)}
    <Case label={`doctor · scanning, ${name} at ${String(width)}`} {width}>
      <Scanning {scanning} />
    </Case>
  {/each}
{/each}
{#each WIDTHS as width (width)}
  <Case label={`doctor · the copy key beside a command and an oid at ${String(width)}`} {width}>
    <div class="flex flex-col gap-snug">
      {#each COPIED as text (text)}
        <div class="flex min-w-0 items-start gap-tight">
          <code class="min-w-0 flex-1 break-all rounded-control bg-chrome px-snug py-tight font-mono text-note text-text-quiet">{text}</code>
          <Copy {text} />
        </div>
      {/each}
    </div>
  </Case>
{/each}

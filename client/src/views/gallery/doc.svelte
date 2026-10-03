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
          verdict: "behind",
          registries: [
            { registry: "npm", reading: { read: { newest: NEWEST } } },
            { registry: "crates_io", reading: "unasked" },
          ],
          update: { channel: "npm", command: "npm install -g sprawling@latest" },
        },
      },
    ],
    [
      "current · installed through cargo",
      {
        stands: {
          mine: NEWEST,
          verdict: "current",
          registries: [
            { registry: "npm", reading: { read: { newest: NEWEST } } },
            { registry: "crates_io", reading: { read: { newest: NEWEST } } },
          ],
          update: { channel: "cargo", command: "cargo install sprawling --locked" },
        },
      },
    ],
    [
      "built from source · npm refused",
      {
        unreleased: {
          registries: [
            { registry: "npm", reading: { refused: { refusal: REFUSAL } } },
            { registry: "crates_io", reading: "unasked" },
          ],
          update: { channel: "source" },
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

  const WIDTHS = [1040, 390] as const;
</script>

<script lang="ts">
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

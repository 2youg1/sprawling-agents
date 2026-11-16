<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  import type { DoctorAnswer, DoctorNewest } from "../../wire";
  import type { Walk } from "../setup/installing";

  const MACHINE: DoctorAnswer = {
    core: "raised",
    custody: { keeps: "across_reboots", store: "platform_service" },
    scanning: "does_not_apply",
    sandbox: {
      arm: "windows_job_object",
      named: "native",
      coverage: [
        { axis: "filesystem", kept: "kept" },
        { axis: "process_tree", kept: "kept" },
        { axis: "resources", kept: "kept" },
        { axis: "network", kept: "not_kept" },
        { axis: "user", kept: "not_kept" },
      ],
    },
    items: [
      {
        name: "git",
        tier: "use",
        need: "required",
        state: {
          present: {
            at: "/usr/bin/git",
            version: { said: { text: "git version 2.55.0" } },
          },
        },
        install: { command: { spelled: "winget install --id Git.Git" } },
      },
      {
        name: "firefox",
        tier: "use",
        need: "required",
        state: { absent: { absence: "not_on_search_path" } },
        install: { command: { spelled: "winget install --id Mozilla.Firefox" } },
      },
      {
        name: "ripgrep",
        tier: "use",
        need: "required",
        state: { absent: { absence: "not_on_search_path" } },
        install: { manual: { how: "download the release archive and put rg on PATH" } },
      },
      {
        name: "geckodriver",
        tier: "use",
        need: "required",
        state: { present: { at: "/usr/bin/geckodriver", version: "silent" } },
        install: { command: { spelled: "cargo install geckodriver" } },
      },
      {
        name: "msedgedriver",
        tier: "use",
        need: "required",
        state: { absent: { absence: "not_on_search_path" } },
        install: { manual: { how: "download the driver that matches the installed Edge" } },
      },
      {
        name: "chromedriver",
        tier: "use",
        need: "optional",
        state: { absent: { absence: "not_on_search_path" } },
        install: "unknown_platform",
      },
      {
        name: "just",
        tier: "develop",
        need: "required",
        state: { present: { at: "/usr/bin/just", version: { said: { text: "just 1.58.0" } } } },
        install: { command: { spelled: "winget install --id Casey.Just -e" } },
      },
      {
        name: "rust",
        tier: "develop",
        need: "required",
        pinned: "1.97.1",
        state: {
          present: {
            at: "/usr/bin/rustc",
            version: { said: { text: "rustc 1.97.1 (8bab26f4f 2026-07-14)" } },
          },
        },
        install: { command: { spelled: "rustup default stable" } },
      },
      {
        name: "cargo-nextest",
        tier: "develop",
        need: "required",
        pack: "rust_tools",
        state: {
          present: {
            at: "/usr/bin/cargo-nextest",
            version: { said: { text: "cargo-nextest 0.9.143" } },
          },
        },
        install: { command: { spelled: "cargo install cargo-nextest --locked" } },
      },
      {
        name: "cargo-deny",
        tier: "develop",
        need: "optional",
        pack: "rust_tools",
        state: { absent: { absence: "not_on_search_path" } },
        install: { command: { spelled: "cargo install cargo-deny --locked" } },
      },
      {
        name: "kani",
        tier: "develop",
        need: "optional",
        pack: "rust_tools",
        state: { absent: { absence: "not_on_search_path" } },
        install: {
          manual: { how: "kani runs on Linux; a WSL installation is where it goes here" },
        },
      },
      {
        name: "elan",
        tier: "develop",
        need: "optional",
        state: { absent: { absence: "not_on_search_path" } },
        install: {
          print: {
            spelled:
              "curl https://raw.githubusercontent.com/leanprover/elan/master/elan-init.sh -sSf | sh",
          },
        },
      },
    ],
    tiers: [
      { tier: "use", missing: ["firefox", "ripgrep"] },
      { tier: "develop", missing: [] },
    ],
  };

  const WALK: Walk = [
    { name: "just", state: "done", why: null },
    {
      name: "cargo-mutants",
      state: "failed",
      why: "read what it reported in the log, then run the line yourself in a terminal",
    },
    { name: "cargo-deny", state: "running", why: null },
  ];

  const NEWEST: Readonly<Record<string, DoctorNewest>> = {
    git: { read: { version: "2.55.1" } },
    just: { read: { version: "1.58.0" } },
    rust: { read: { version: "1.98.0" } },
    "cargo-nextest": { read: { version: "0.9.143" } },
    "cargo-deny": { read: { version: "0.19.2" } },
    kani: { refused: { said: "crates.io answered 503 Service Unavailable" } },
    geckodriver: { unread: { why: "matches_browser" } },
    elan: "asking",
    msedgedriver: { unread: { why: "matches_browser" } },
  };
</script>

<script lang="ts">
  import { MachineReport, MachineUnchecked } from "../machine.svelte";
  import Setup from "../setup.svelte";
  import Decided from "../setup/decided.svelte";
  import { TimeMs, type Decision, type Answer, type Query } from "../../wire";
  import { QUERIES } from "../../core/asking";
  import { ENDPOINTS } from "./served";
  import Case from "./case.svelte";
  import { answered } from "./shelved.svelte";
  import Stand from "./stand.svelte";

  function ruled(item: string, detail: string, at: number, verdict: Decision["verdict"]): Decision {
    return { at: TimeMs.make(at), cluster: { class: "question", detail }, item, verdict };
  }
  function performanceAnswer(query: Query): Answer | undefined {
    return query === QUERIES.preferences
      ? {
          preferences: {
            core: { placement: "soft_shares", priority: "normal", memory_bytes: null },
          },
        }
      : undefined;
  }
  function cappedAnswer(query: Query): Answer | undefined {
    return query === QUERIES.preferences
      ? {
          preferences: {
            core: { placement: "soft_shares", priority: "raised", memory_bytes: 8_589_934_592 },
          },
        }
      : undefined;
  }

  const CLERK: readonly Decision[] = [
    ruled("ap-1", "keep the ledger schema at v3 for this release", 1_767_225_600_000, "allow"),
    ruled("ap-2", "split the gate module before adding the dedup rule", 1_767_229_200_000, "allow"),
    ruled("ap-3", "drop the legacy wire frame in this change", 1_767_232_800_000, "deny"),
  ];
</script>

{#snippet cards()}
  <MachineReport
    answer={MACHINE}
    onInstall={() => undefined}
    planned={["cargo-deny"]}
    onInstallAll={() => undefined}
    walk={WALK}
    newest={NEWEST}
    onInstallPack={() => undefined}
  />
{/snippet}

{#snippet unchecked()}
  <MachineUnchecked onRecheck={() => undefined} />
{/snippet}

<Case label="settings · accounts, and the models the city may call" width={1440}>
  <Setup
    group="accounts"
    endpoints={ENDPOINTS}
    autonomy={undefined}
    onAutonomy={() => undefined}
    dependency={unchecked}
  />
</Case>

<Case label="settings · dependencies, one card per program" width={1440}>
  <Setup
    group="tools"
    endpoints={undefined}
    autonomy={undefined}
    onAutonomy={() => undefined}
    dependency={cards}
  />
</Case>

<Case label="settings · dependencies, nobody has looked yet" width={1440}>
  <Setup
    group="tools"
    endpoints={undefined}
    autonomy={undefined}
    onAutonomy={() => undefined}
    dependency={unchecked}
  />
</Case>

<Case label="settings · about, one row and the check that is never automatic" width={1440}>
  <Setup
    group="about"
    endpoints={undefined}
    autonomy={undefined}
    onAutonomy={() => undefined}
    dependency={unchecked}
  />
</Case>

<Case label="setup · accounts at 1280" width={1280}>
  <Setup
    group="accounts"
    endpoints={ENDPOINTS}
    autonomy={undefined}
    onAutonomy={() => undefined}
    dependency={unchecked}
  />
</Case>

<Case label="setup · accounts at 820" width={820}>
  <Setup
    group="accounts"
    endpoints={ENDPOINTS}
    autonomy={undefined}
    onAutonomy={() => undefined}
    dependency={unchecked}
  />
</Case>

<Case label="setup · tools at 1280" width={1280}>
  <Setup
    group="tools"
    endpoints={undefined}
    autonomy={undefined}
    onAutonomy={() => undefined}
    dependency={cards}
  />
</Case>

<Case label="setup · tools at 820" width={820}>
  <Setup
    group="tools"
    endpoints={undefined}
    autonomy={undefined}
    onAutonomy={() => undefined}
    dependency={cards}
  />
</Case>

<Case label="setup · skills at 1280" width={1280}>
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answered}>
    <Setup
      group="skills"
      endpoints={undefined}
      autonomy={undefined}
      onAutonomy={() => undefined}
      dependency={unchecked}
    />
  </Stand>
</Case>

<Case label="setup · skills at 820" width={820}>
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answered}>
    <Setup
      group="skills"
      endpoints={undefined}
      autonomy={undefined}
      onAutonomy={() => undefined}
      dependency={unchecked}
    />
  </Stand>
</Case>

<Case label="setup · answered for you, nothing yet" width={820}>
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answered}>
    <Decided decided={[]} />
  </Stand>
</Case>

<Case label="setup · answered for you, three by the clerk" width={820}>
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answered}>
    <Decided decided={CLERK} />
  </Stand>
</Case>

<Case label="settings · performance, no memory ceiling" width={820}>
  <Stand
    link={{ kind: "live", city: "sprawling" }}
    unread={[]}
    waiting={[]}
    answers={performanceAnswer}
  >
    <Setup group="performance" />
  </Stand>
</Case>

<Case label="settings · performance, an entered memory ceiling" width={820}>
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={cappedAnswer}>
    <Setup group="performance" />
  </Stand>
</Case>

<Case label="settings · performance on one column" width={390}>
  <Stand
    link={{ kind: "live", city: "sprawling" }}
    unread={[]}
    waiting={[]}
    answers={performanceAnswer}
  >
    <Setup group="performance" />
  </Stand>
</Case>

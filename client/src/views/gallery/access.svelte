<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts" module>
  // The ways in and the agents that come in, in every state they take:
  // the ACP agents page at rest, with a paste read by the city, and with
  // no detected agent; the pairing page for a first browser and after a
  // used link; and the paired browsers on this machine, from the remote
  // door, and with none.

  import type { AgentCatalogAnswer, AgentOffer, DeviceLine } from "../../wire";
  import { B3Hash, DeviceId, TimeMs } from "../../wire";

  const DIGEST = B3Hash.make("0".repeat(64));
  const NOW = 1_791_500_000_000;

  function offer(id: string, name: string, more: Partial<AgentOffer>): AgentOffer {
    return {
      id,
      name,
      source: "registry",
      launch_preview: `npx -y ${id}`,
      env_names: [],
      login: [],
      pinned: "exact",
      spec_digest: DIGEST,
      ...more,
    };
  }

  const KIMI = offer("kimi", "Kimi Code", {
    source: "detected",
    launch_preview: "kimi acp",
    version: "0.41.0",
    pinned: "floating",
    licence: "Apache-2.0",
    login: ["terminal"],
  });
  const PI = offer("pi-acp", "Pi", {
    source: "detected",
    launch_preview: "npx -y pi-acp@0.0.34",
    version: "0.0.34",
    licence: "MIT",
    login: ["agent"],
  });

  const CATALOG: AgentCatalogAnswer = {
    detected: [KIMI, PI],
    catalog: [
      offer("gemini", "Gemini CLI", { version: "0.9.0", licence: "Apache-2.0", env_names: ["GEMINI_API_KEY"] }),
      offer("goose", "Goose", { version: "1.9.0", licence: "Apache-2.0" }),
      offer("opencode", "OpenCode", { version: "0.15.2", licence: "MIT" }),
      offer("auggie", "Auggie", { version: "0.5.10" }),
    ],
    added: [
      {
        id: "claude-acp",
        name: "Claude Code",
        source: "registry",
        version: "0.88.0",
        pinned: "exact",
        login_state: "required",
        auth_methods: [{ id: "console-login", name: "Console", kind: "terminal" }],
        seated_in: [],
      },
      {
        id: "codex-acp",
        name: "Codex",
        source: "registry",
        version: "0.6.0",
        pinned: "exact",
        login_state: "ready",
        auth_methods: [],
        seated_in: [],
      },
    ],
    snapshot: { date: "2026-10-09", etag: "W/\"fixture\"" },
  };

  const BARE: AgentCatalogAnswer = { ...CATALOG, detected: [], added: [] };

  const PASTED = offer("my-agent", "my-agent", {
    source: "pasted",
    launch_preview: "/usr/local/bin/my-agent --acp",
    pinned: "unknown",
    env_names: ["MY_AGENT_TOKEN"],
  });

  const DEVICES: readonly DeviceLine[] = [
    { id: DeviceId.make("d-one"), label: "Firefox · Windows", paired_at: TimeMs.make(NOW - 86_400_000 * 3), last_seen: TimeMs.make(NOW - 120_000) },
    { id: DeviceId.make("d-two"), label: "Safari · macOS", paired_at: TimeMs.make(NOW - 3_600_000) },
  ];
</script>

<script lang="ts">
  import { readable } from "svelte/store";

  import type { Lang } from "../../core/lang";
  import { ui } from "../../ui";
  import Pairing from "../pairing.svelte";
  import Devices from "../settings/devices_list.svelte";
  import { readingOf } from "../setup/agents";
  import Agents from "../setup/agents_page.svelte";
  import Case from "./case.svelte";

  const { lang } = ui();
  const still = () => undefined;
  const never = () => Promise.resolve({ kind: "unreachable" } as const);
  const NONE: ReadonlySet<string> = new Set();
</script>

<Case label="acp agents · at rest, two found here, two added">
  <Agents
    answer={CATALOG}
    text=""
    reading={readingOf("", CATALOG)}
    parsed={null}
    room="hall/mayor"
    adding={NONE}
    onText={still}
    onPaste={still}
    onAdd={still}
    onLogin={still}
  />
</Case>
<Case label="acp agents · a pasted command line, read by the city">
  <Agents
    answer={CATALOG}
    text={PASTED.launch_preview}
    reading={readingOf(PASTED.launch_preview, CATALOG)}
    parsed={PASTED}
    room="hall/mayor"
    adding={NONE}
    onText={still}
    onPaste={still}
    onAdd={still}
    onLogin={still}
  />
</Case>
<Case label="acp agents · nothing found here, nothing added, no room">
  <Agents
    answer={BARE}
    text=""
    reading={readingOf("", BARE)}
    parsed={null}
    room={null}
    adding={NONE}
    onText={still}
    onPaste={still}
    onAdd={still}
    onLogin={still}
  />
</Case>
<Case label="pairing · a first browser">
  <Pairing why="first" {lang} label="Firefox · Windows" pair={never} onPaired={still} seat="specimen" />
</Case>
<Case label="pairing · after a used link">
  <Pairing why="open_used" lang={readable<Lang>("zh")} label="Edge · Windows" pair={never} onPaired={still} seat="specimen" />
</Case>
<Case label="paired browsers · on this machine, this browser marked">
  <Devices devices={DEVICES} here="d-one" now={NOW} local onForget={still} />
</Case>
<Case label="paired browsers · from the remote door">
  <Devices devices={DEVICES} here={null} now={NOW} local={false} onForget={still} />
</Case>
<Case label="paired browsers · none">
  <Devices devices={[]} here={null} now={NOW} local onForget={still} />
</Case>

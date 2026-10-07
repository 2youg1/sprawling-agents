<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts" module>
  // The inspector (docs/frontend-method.md §7F, client/Spec.lean §4-45), at the width the right side
  // takes beside the conversation in zen on a 1440 window: one run that
  // read a file, changed it, ran the tests, is running a second command,
  // took a screenshot and failed a search, in a made-up city that
  // answers the rounds, the patch, the file and the original output.
  // Each case is the side following that run with a different pair of
  // calls in front, because what a person opened is one state for the
  // whole page and a fixture must not write it.
  import type { Answer, Call, Query, RoundsAnswer, Turn } from "../../wire";
  import { Address, B3Hash, GitOid, Locator, RunId, Seq, TimeMs } from "../../wire";
  import type { RightItem } from "../inspect/open.svelte";
  import { lookOf as patchOf } from "../inspect/patch";
  import { lookOf as splitOf } from "../inspect/split";
  import { lookOf as stripOf } from "../inspect/strip";
  import type { TextKeyLook } from "../inspect/text_key";

  const ROOM = Address.make("release/ledger");
  const RUN = RunId.make("0199c0de-5a6b-4c3d-8e4f-000000000123");
  const START = 1_790_000_000_000;
  const PATH = "crates/city/src/document.rs";
  const OPENED = GitOid.make("7c41e09aa1b2c3d4e5f60718293a4b5c6d7e8f90");
  const CHECKED = GitOid.make("a1b2c3d4e5f60718293a4b5c6d7e8f9012345678");
  const PINNED = Locator.make(`cas:b3-${"5e".repeat(32)}`);
  const PICTURE = Locator.make(`cas:b3-${"c3".repeat(32)}`);

  function call(at: number, took: number | null, rest: Partial<Call> & Pick<Call, "tool">): Call {
    return {
      subject: null,
      arguments: null,
      outcome: took === null ? "waiting" : "answered",
      at: Seq.make(at),
      output: null,
      called: TimeMs.make(START + at * 1_000),
      answered: took === null ? null : TimeMs.make(START + at * 1_000 + took),
      timing: "measured",
      ...rest,
    };
  }

  const FILE = `/// A document of a building, at a version the ledger recorded.
pub(crate) struct Document {
    path: Address,
    held: Held,
}

impl Document {
    /// Writes \`edits\` against the version the person saw, or refuses.
    pub(crate) fn edit_against(
        &self,
        baseline: B3Hash,
        edits: &[Edit],
    ) -> Result<Receipt, AxError> {
        let held = self.version();
        if held != baseline {
            return Err(stale(self.path(), held, baseline));
        }
        let next = self.apply(edits)?;
        Ok(Receipt::written(next.version()))
    }
}
`;

  const READ = call(11, 14, {
    tool: "read",
    subject: PATH,
    effect: "read",
    render: "generic",
    arguments: { head: JSON.stringify({ path: PATH, offset: 13 }, null, 2), cut: 0 },
    output: { head: JSON.stringify({ path: PATH, text: FILE, bytes: FILE.length }), cut: 0 },
  });
  const EDIT = call(12, 31, {
    tool: "edit",
    subject: PATH,
    effect: { write: { domain: ROOM } },
    render: { diff: { locations: [] } },
    output: { head: JSON.stringify({ path: PATH, base_version: "b3", new_version: "b3", diff: "" }), cut: 0 },
  });
  const TESTS = call(21, 3_412, {
    tool: "exec",
    effect: "egress",
    render: "terminal",
    arguments: { head: JSON.stringify({ arm: { shell: { text: "cargo nextest -p city" } } }), cut: 0 },
    output: {
      head: JSON.stringify({
        stdout: [
          "    Starting 41 tests across 3 binaries",
          "        \u001b[32mPASS\u001b[0m [   0.031s] city document::tests::edit_against_refuses_a_stale_baseline",
          "        \u001b[32mPASS\u001b[0m [   0.029s] city document::tests::edit_against_writes_on_the_seen_version",
          "        \u001b[32mPASS\u001b[0m [   0.112s] city library::tests::a_shelf_outside_the_city_is_read_only",
          "     Summary [   3.412s] 41 tests run: \u001b[32m41 passed\u001b[0m, 0 skipped",
        ].join("\n"),
        stderr: "",
        exit_code: 0,
      }),
      cut: 214,
      pinned: PINNED,
    },
  });
  const CLIPPY = call(22, null, {
    tool: "exec",
    effect: "egress",
    render: "terminal",
    arguments: { head: JSON.stringify({ arm: { shell: { text: "cargo clippy -p city --all-targets -- -D warnings" } } }), cut: 0 },
  });
  const SHOT = call(23, 880, {
    tool: "browser",
    subject: "screenshot",
    effect: "egress",
    render: "generic",
    output: { head: JSON.stringify({ image: PICTURE, width: 1280, height: 720, media_type: "image/png" }), cut: 0 },
  });
  const SEARCH = call(24, 9, {
    tool: "search",
    subject: "crates/nowhere",
    effect: "read",
    render: "generic",
    outcome: "failed",
    arguments: { head: JSON.stringify({ pattern: "edit_against", path: "crates/nowhere" }, null, 2), cut: 0 },
    output: { head: "no such directory: crates/nowhere", cut: 0 },
  });

  function turn(number: number, calls: readonly Call[], notes: Turn["notes"]): Turn {
    return { calls, notes, number, opened: Seq.make(number * 10), t: TimeMs.make(START + number * 10_000), timing: "measured" };
  }

  const ROUNDS: RoundsAnswer = {
    run: RUN,
    opened_at: OPENED,
    turns: [
      turn(1, [READ, EDIT], [{ checkpointed: { at: Seq.make(13), oid: CHECKED } }]),
      turn(2, [TESTS, CLIPPY, SHOT, SEARCH], []),
    ],
  };

  const PATCH = [
    `diff --git a/${PATH} b/${PATH}`,
    "@@ -38,13 +38,13 @@ impl Document {",
    "     /// Writes `edits` against the version the person saw, or refuses.",
    "     pub(crate) fn edit_against(",
    "         &self,",
    "-        baseline: Option<B3Hash>,",
    "+        baseline: B3Hash,",
    "         edits: &[Edit],",
    "     ) -> Result<Receipt, AxError> {",
    "-        if let Some(seen) = baseline {",
    "+        let held = self.version();",
    "+        if held != baseline {",
    "             return Err(stale(self.path(), held, baseline));",
    "         }",
  ];

  // What the made-up city answers; `moved` says whether the file has
  // changed in the worktree since the checkpoint the patch reads into.
  function answering(moved: boolean): (query: Query) => Answer | undefined {
    return (query) => {
      if (typeof query !== "object") return undefined;
      if ("rounds" in query) return { rounds: ROUNDS };
      if ("hunks" in query) {
        return {
          hunks: { oid_a: OPENED, oid_b: CHECKED, path: PATH, lines: PATCH.map((text, at) => ({ number: at + 1, text })), withheld: [] },
        };
      }
      if ("changes" in query) {
        return { changes: { base: CHECKED, head: null, files: moved ? [{ path: PATH, how: "modified", lines: { counted: { added: 2, removed: 1 } } }] : [] } };
      }
      if ("document" in query) {
        return {
          document: {
            at: query.document.at,
            state: {
              held: {
                body: { text: { coverage: "whole", encoding: "utf8", head: { span: { start: 0, end: FILE.length }, text: FILE } } },
                bytes: FILE.length,
                format: "plain",
                version: B3Hash.make("b3".repeat(32)),
              },
            },
          },
        };
      }
      if ("content" in query) {
        return { content: { locator: query.content.locator, binary: false, bytes: 48_211, truncated: true, text: "    Starting 41 tests across 3 binaries\n" } };
      }
      return undefined;
    };
  }

  function item(at: Call): RightItem {
    return { kind: "call", run: RUN, at: at.at };
  }

  interface Shown {
    readonly label: string;
    readonly following: readonly RightItem[];
    readonly moved: boolean;
  }

  // The strip, the line between the regions, the worded keys and a
  // patch with a chosen line, drawn from their looks with fixed values:
  // what a person opened is one state for the whole page, so the states
  // a follow cannot reach - a tab behind with its close mark, a draft
  // the city has not taken, an item with no link, a toggle held down -
  // are drawn here rather than by opening items.
  const NOTHING = (): undefined => undefined;
  const STRIP = stripOf(
    [
      { key: "a", label: "document.rs", terminal: false, unsaved: false, front: false, href: "#/talk/release/ledger", controls: "fixture-editor" },
      { key: "b", label: "notes.md", terminal: false, unsaved: true, front: true, href: "#/talk/release/ledger", controls: "fixture-editor" },
      { key: "c", label: "cargo nextest -p city", terminal: true, unsaved: false, front: false, href: "#/talk/release/ledger", controls: "fixture-terminal" },
      { key: "d", label: "7c41e09", terminal: false, unsaved: false, front: false, href: null, controls: "fixture-editor" },
    ],
    { tabs: "open in the inspector", unsaved: "unsaved", closeAll: "close the inspector", closeItem: (name) => `close ${name}` },
    { pick: NOTHING, close: NOTHING, closeAll: NOTHING, focus: NOTHING, hold: () => NOTHING },
  );
  const LINE = splitOf(
    { lines: 12, least: 6, most: 30, controls: "fixture-editor", step: 24, onLines: NOTHING, onReset: NOTHING },
    "move the line between the editor and the terminal",
    { capture: NOTHING, hold: NOTHING, grip: { from: null } },
  );
  const KEYS: readonly TextKeyLook[] = [
    { label: "editor", wire: { href: "#/gallery" } },
    { label: "copy the place", wire: { type: "button", onclick: NOTHING } },
    { label: "original", wire: { type: "button", "aria-pressed": false, onclick: NOTHING } },
    { label: "original", wire: { type: "button", "aria-pressed": true, onclick: NOTHING } },
  ];
  const CHOSEN = patchOf(
    { oid_a: OPENED, oid_b: CHECKED, path: PATH, lines: PATCH.map((text, at) => ({ number: at + 1, text })), withheld: [{ number: 99, reason: "a key-shaped value" }] },
    7,
    { withheld: (n, reason) => `line ${n} withheld: ${reason}`, folded: (n) => `${n} lines folded`, quote: (n) => `quote line ${n}` },
    { href: "#/gallery", choose: NOTHING },
  );

  const SHOWN: readonly Shown[] = [
    { label: "inspector · a diff above the command that tested it", following: [item(EDIT), item(TESTS)], moved: false },
    { label: "inspector · a diff the worktree has moved past", following: [item(EDIT)], moved: true },
    { label: "inspector · the file a call read, above a command still running", following: [item(READ), item(CLIPPY)], moved: false },
    { label: "inspector · a screenshot, above a search that failed", following: [item(SHOT), item(SEARCH)], moved: false },
  ];
</script>

<script lang="ts">
  import PatchLook from "../inspect/patch.look.svelte";
  import SplitLook from "../inspect/split.look.svelte";
  import StripLook from "../inspect/strip.look.svelte";
  import TextKey from "../inspect/text_key.look.svelte";
  import Right from "../right.svelte";
  import Case from "./case.svelte";
  import Stand from "./stand.svelte";
</script>

{#each SHOWN as shown (shown.label)}
  <Case label={shown.label} width={720}>
    <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answering(shown.moved)}>
      <div class="flex h-[720px] border-l border-edge-panel">
        <Right following={shown.following} current={undefined} talk={ROOM} />
      </div>
    </Stand>
  </Case>
{/each}

<Case label="inspector · the strip, the line, the worded keys and a chosen patch line" width={720}>
  <div class="flex flex-col bg-chrome">
    <StripLook {...STRIP} />
    <div id="fixture-terminal" class="flex items-center gap-base px-wide py-snug">
      {#each KEYS as key, at (at)}
        <TextKey {...key} />
      {/each}
    </div>
    <SplitLook {...LINE} />
    <div id="fixture-editor" class="overflow-auto bg-page">
      <PatchLook {...CHOSEN} />
    </div>
  </div>
</Case>

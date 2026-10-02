<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts" module>
  // The right side keeping its place (client/Spec.lean §4-63), in a made-up
  // city: a document tab whose RefRain session holds a draft beside a
  // commit's changes, the changes a commit picked in the workbench
  // opens on the right, and the line that says a draft the browser
  // would not store lives in this tab alone. The cases follow items
  // rather than open them, because what a person opened is one state for
  // the whole page and a fixture must not write it.
  import { draftPlace, writeDraft } from "../../core/document_save";
  import { Address, B3Hash, GitOid } from "../../wire";
  import type { Answer, Query } from "../../wire";
  import type { RightItem } from "../inspect/open.svelte";

  const LAB = Address.make("lab");
  const PATH = "notes/plan.md";
  const VERSION = B3Hash.make("3c1f0a9e".repeat(8));
  const BASE = GitOid.make("7c41e09aa1b2c3d4e5f60718293a4b5c6d7e8f90");
  const HEAD = GitOid.make("a1b2c3d4e5f60718293a4b5c6d7e8f9012345678");
  const CHANGED = "crates/city/src/document.rs";

  const PLAN = "# Plan\n\nThe reader opens a version, edits it in the browser, and saves it.\n";

  const PATCH = [
    "@@ -12,4 +12,5 @@ impl Document {",
    "         let held = self.version();",
    "-        if held == baseline {",
    "+        if held != baseline {",
    "+            return Err(stale(self.path(), held, baseline));",
    "         }",
  ];

  function answering(query: Query): Answer | undefined {
    if (typeof query !== "object") return undefined;
    if ("document" in query) {
      const text = { coverage: "whole" as const, encoding: "utf8" as const, head: { span: { start: 0, end: PLAN.length }, text: PLAN } };
      return { document: { at: query.document.at, state: { held: { body: { text }, bytes: PLAN.length, format: "markdown", version: VERSION } } } };
    }
    if ("changes" in query) {
      return {
        changes: {
          base: query.changes.base,
          head: query.changes.head ?? null,
          files: [
            { path: CHANGED, how: "modified", lines: { counted: { added: 2, removed: 1 } } },
            { path: PATH, how: "added", lines: { counted: { added: 3, removed: 0 } } },
          ],
        },
      };
    }
    if ("hunks" in query) {
      return { hunks: { oid_a: BASE, oid_b: HEAD, path: CHANGED, lines: PATCH.map((text, at) => ({ number: at + 1, text })), withheld: [] } };
    }
    return undefined;
  }

  const DOCUMENT: RightItem = { kind: "document", building: LAB, path: PATH, version: null };
  const CHANGES: RightItem = { kind: "changes", base: BASE, head: HEAD };
  const KEPT = writeDraft({ version: VERSION, changes: [{ from: PLAN.length, to: PLAN.length, insert: "\nA line not saved yet.\n" }] });
</script>

<script lang="ts">
  import { ui } from "../../ui";
  import Unkept from "../parts/unkept.svelte";
  import Right from "../right.svelte";
  import Case from "./case.svelte";
  import Stand from "./stand.svelte";

  // The draft the first case's RefRain restores, written where it looks
  // for one before it mounts.
  ui().prefs.setDraft(draftPlace(Address.make(`${LAB}/${PATH}`)), KEPT);
</script>

<Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answering}>
  <Case label="inspector · a document tab holding an unsaved draft, beside a commit's changes" width={720}>
    <div class="flex h-[480px] border-l border-edge-panel">
      <Right following={[DOCUMENT, CHANGES]} current={undefined} talk={LAB} />
    </div>
  </Case>
  <Case label="workbench · a picked commit's changes on the right" width={560}>
    <div class="flex h-[480px] border-l border-edge-panel">
      <Right following={[CHANGES]} current={undefined} talk={LAB} />
    </div>
  </Case>
  <Case label="draft · the browser would not store it" width={760}>
    <Unkept words={() => "A sentence the quota refused."} />
  </Case>
</Stand>

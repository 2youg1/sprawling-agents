// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import { decodeWorkspaceText, encodeWorkspaceChanges, proposalBlock } from "./workspace_text";

describe("registered workspace text", () => {
  test("keeps a UTF-8 BOM and untouched mixed line endings", () => {
    const encoder = new TextEncoder();
    const original = encoder.encode("\uFEFF甲\r\n乙\n丙\r丁");
    const baseline = decodeWorkspaceText("base", original);
    expect(baseline?.editor).toBe("甲\n乙\n丙\n丁");
    if (baseline === null) return;
    const edited = encodeWorkspaceChanges(original, baseline, [
      { from: 2, to: 3, insert: "戊\n己" },
    ]);
    expect(edited).toEqual(encoder.encode("\uFEFF甲\r\n戊\r\n己\n丙\r丁"));
  });

  test("keeps UTF-16 byte order and puts replacements on encoded boundaries", () => {
    const original = Uint8Array.of(0xfe, 0xff, 0xd8, 0x3d, 0xde, 0x00, 0x00, 0x0d, 0x4e, 0x59);
    const baseline = decodeWorkspaceText("base", original);
    expect(baseline?.editor).toBe("😀\n乙");
    if (baseline === null) return;
    const edited = encodeWorkspaceChanges(original, baseline, [{ from: 0, to: 2, insert: "甲" }]);
    expect(edited).toEqual(Uint8Array.of(0xfe, 0xff, 0x75, 0x32, 0x00, 0x0d, 0x4e, 0x59));
  });

  test("refuses opaque or invalid sequences", () => {
    expect(decodeWorkspaceText("base", Uint8Array.of(0x41, 0x00))).toBeNull();
    expect(decodeWorkspaceText("base", Uint8Array.of(0x81))).toBeNull();
    expect(decodeWorkspaceText("base", Uint8Array.of(0xff, 0xfe, 0x00))).toBeNull();
  });
});

describe("registered proposal baseline", () => {
  const proposal = {
    workspace: "w",
    file: "f",
    version: "base",
    label: "research.md",
    from: 0,
    to: 2,
    quote: "😀",
    replacement: "甲",
  };

  test("allows the exact quoted range even when there are disjoint local edits", () => {
    const baseline = decodeWorkspaceText("base", new TextEncoder().encode("😀 title"));
    expect(proposalBlock(proposal, baseline, [{ from: 3, to: 4, insert: "T" }])).toBeNull();
    expect(proposalBlock(proposal, baseline, [{ from: 2, to: 2, insert: "!" }])).toEqual("overlap");
  });

  test("refuses a changed version, a changed quote, or a range split within an encoded character", () => {
    const baseline = decodeWorkspaceText("base", new TextEncoder().encode("😀 title"));
    expect(proposalBlock({ ...proposal, version: "other" }, baseline, [])).toEqual("version");
    expect(proposalBlock({ ...proposal, quote: "unrelated" }, baseline, [])).toEqual("invalid");
    expect(proposalBlock({ ...proposal, to: 1, quote: "\ud83d" }, baseline, [])).toEqual("invalid");
  });
});

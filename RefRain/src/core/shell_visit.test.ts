// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { expect, test } from "bun:test";

import { Address, Seq } from "../wire";
import { toFragment, type WorkspaceView } from "./route";
import { memory } from "./rows";
import { launchView, shellVisit } from "./shell_visit";

test("a plain launch returns to the recorded subject, geometry, file and session", () => {
  const stored = memory();
  const first = shellVisit(stored);
  const chosen: WorkspaceView = {
    kind: "talk",
    address: Address.make("lab/notes"),
    session: Seq.make(12),
  };
  first.setSubject("files");
  first.setWorkspace("root");
  first.setFile("root", "paper");
  first.setFile("other", "copy");
  first.setSplit(true);
  first.setSplitLine(7);
  expect(first.splitTop()).toBe(50);
  first.setSplitTop(63);
  first.setTalk(chosen);

  const cold = shellVisit(stored);
  expect(launchView("", cold)).toEqual(chosen);
  expect(cold.subject()).toBe("files");
  expect(cold.split()).toBe(true);
  expect(cold.splitLine()).toBe(7);
  expect(cold.splitTop()).toBe(63);
  expect(cold.workspace()).toBe("root");
  expect(cold.file("root")).toBe("paper");
  expect(cold.file("other")).toBe("copy");

  const explicit: WorkspaceView = {
    kind: "talk",
    address: Address.make("lab/reference"),
    item: { kind: "workspace-file", workspace: "other", file: "different" },
  };
  expect(launchView(toFragment(explicit), cold)).toEqual(explicit);
});

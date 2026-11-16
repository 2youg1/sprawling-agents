// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The skills page's folder box, against the paths a person actually
// types: a Windows drive, a POSIX path, a home-relative one, a UNC share,
// and the two shapes the city would refuse.

import { describe, expect, test } from "bun:test";

import { alreadyMounted, offered, withLibrary, withoutLibrary } from "./skill_libraries";

describe("a skill library a person points at", () => {
  test("an absolute path on either platform is offered, and a home-relative one is the city's to expand", () => {
    expect(
      ["", "  ", "C:\\work\\skills", "/work/skills", "~/skills", "\\\\host\\share\\skills"].map(
        offered,
      ),
    ).toEqual([
      { kind: "blank" },
      { kind: "blank" },
      { kind: "path", path: "C:\\work\\skills" },
      { kind: "path", path: "/work/skills" },
      { kind: "path", path: "~/skills" },
      { kind: "path", path: "\\\\host\\share\\skills" },
    ]);
  });

  test("a relative path and the bare home marker are refused before the frame leaves", () => {
    expect([offered("skills"), offered("."), offered("~"), offered("~/")]).toEqual([
      { kind: "refused", why: "skill_mount_relative" },
      { kind: "refused", why: "skill_mount_relative" },
      { kind: "refused", why: "skill_mount_home" },
      { kind: "refused", why: "skill_mount_home" },
    ]);
  });

  test("adding keeps the order and never mounts one folder twice, and removing takes it out by spelling", () => {
    const start = ["/one", "~/two"];
    const added = withLibrary(start, "/three");
    expect([start, added, withLibrary(added, " /three "), alreadyMounted(added, "/three")]).toEqual(
      [["/one", "~/two"], ["/one", "~/two", "/three"], ["/one", "~/two", "/three"], true],
    );
    expect(withoutLibrary(added, "~/two")).toEqual(["/one", "/three"]);
  });
});

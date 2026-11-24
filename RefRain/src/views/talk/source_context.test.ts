// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { expect, test } from "bun:test";
import type { DocumentAttachment } from "./attachments";
import { groundedAnswer, sourcesOf, withSources, withoutSources } from "./source_context";

const id = "a561a201-e20d-45da-aefd-ead844f2fc6b";
const source: DocumentAttachment = {
  workspace: "literature",
  file: "paper-1",
  label: "paper.md",
  version: "before",
  selection: { kind: "markdown", from: 8, to: 12, text: "同一段落" },
};

test("discussion and accepted-review requests share one stable selected range", () => {
  const message = withSources("Discuss this passage", [{ ...source, id }]);
  expect(withoutSources(message)).toBe("Discuss this passage");
  const selected = sourcesOf(message, []);
  const discussed = groundedAnswer(`The passage says… [[ref:${id}]]`, selected);
  expect(discussed.proposal).toBeNull();
  expect(discussed.citations[0]).toMatchObject(source);
  const rewrite = groundedAnswer(
    `\`\`\`refrain-patch\n{"ref":"${id}","replacement":"这里有两句。它们都很短。"}\n\`\`\``,
    selected,
  );
  expect(rewrite.proposal).toMatchObject({
    workspace: "literature",
    file: "paper-1",
    version: "before",
    from: 8,
    to: 12,
    quote: "同一段落",
    replacement: "这里有两句。它们都很短。",
  });
  expect(
    groundedAnswer(`\`\`\`refrain-patch\n{"ref":"other","replacement":"ignored"}\n\`\`\``, selected)
      .proposal,
  ).toBeNull();
});

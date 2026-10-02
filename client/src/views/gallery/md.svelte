<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // A model's reply as the thread draws it (client-SPEC 4-26, 4-53), on
  // a made-up city that answers `Query::Reply` with the blocks the
  // city's grammar lays out for each text below, written out by hand:
  // a settled reply with every kind of block, a streaming one whose
  // list, table and code block have closed while its last paragraph
  // arrives, a streaming one inside an open code fence, a reply the
  // city refuses to read, and one it has not answered yet.
  import type { Answer, Block, Inline, Query } from "../../wire";

  function bytes(text: string): number {
    return new TextEncoder().encode(text).length;
  }

  // The byte span of `source`, the first time it appears in `text`.
  function spanIn(text: string, source: string): { start: number; end: number } {
    const at = text.indexOf(source);
    const start = bytes(text.slice(0, Math.max(at, 0)));
    return { start, end: start + bytes(source) };
  }

  const words = (text: string): Inline => ({ text });

  // wording-ok: fixture text is a model's reply, not a page's words
  const SETTLED = [
    "## What changed",
    "",
    "The thread now draws **every** block the city lays out, and the page reads *no* Markdown of its own. The grammar is in [the documents spec](https://example.invalid/spec), the plan in [notes/plan.md](notes/plan.md).[^1]",
    "",
    "1. Ask the city with the reply's text",
    "2. Draw what it laid out",
    "",
    "- [x] delete the second grammar",
    "- [ ] read a round trip on the remote door",
    "",
    "| step | where |",
    "| :--- | ---: |",
    "| ask | `core/replying.ts` |",
    "| draw | `refrain/laid.svelte` |",
    "",
    "> A block the city has closed is never taken back.",
    "",
    "```rust",
    "let laid = documents::reply(text, ReplyState::Settled)?;",
    "```",
    "",
    "<details>raw HTML</details>",
    "",
    "[^1]: The closure rule is D31.",
    "",
  ].join("\n");

  const at = (source: string) => spanIn(SETTLED, source);

  const SETTLED_BLOCKS: readonly Block[] = [
    { heading: { level: 2, span: at("## What changed\n"), inline: [words("What changed")] } },
    {
      paragraph: {
        span: at("The thread now"),
        inline: [
          words("The thread now draws "),
          { strong: [words("every")] },
          words(" block the city lays out, and the page reads "),
          { emphasis: [words("no")] },
          words(" Markdown of its own. The grammar is in "),
          { link: { target: "https://example.invalid/spec", title: "", content: [words("the documents spec")] } },
          words(", the plan in "),
          { link: { target: "notes/plan.md", title: "", content: [words("notes/plan.md")] } },
          words("."),
          { footnote_reference: { name: "1" } },
        ],
      },
    },
    {
      list: {
        span: at("1. Ask"),
        order: { ordered: { start: 1 } },
        spacing: "tight",
        items: [
          { span: at("1. Ask"), check: "not_a_task", blocks: [{ paragraph: { span: at("Ask the"), inline: [words("Ask the city with the reply's text")] } }] },
          { span: at("2. Draw"), check: "not_a_task", blocks: [{ paragraph: { span: at("Draw what"), inline: [words("Draw what it laid out")] } }] },
        ],
      },
    },
    {
      list: {
        span: at("- [x]"),
        order: "bullet",
        spacing: "tight",
        items: [
          { span: at("- [x]"), check: "done", blocks: [{ paragraph: { span: at("delete the"), inline: [words("delete the second grammar")] } }] },
          { span: at("- [ ]"), check: "open", blocks: [{ paragraph: { span: at("read a round"), inline: [words("read a round trip on the remote door")] } }] },
        ],
      },
    },
    {
      table: {
        span: at("| step"),
        align: ["left", "right"],
        head: { cells: [[words("step")], [words("where")]] },
        body: [
          { cells: [[words("ask")], [{ code: "core/replying.ts" }]] },
          { cells: [[words("draw")], [{ code: "refrain/laid.svelte" }]] },
        ],
      },
    },
    { quote: { span: at("> A block"), blocks: [{ paragraph: { span: at("A block"), inline: [words("A block the city has closed is never taken back.")] } }] } },
    { code: { span: at("```rust"), info: "rust", text: "let laid = documents::reply(text, ReplyState::Settled)?;\n" } },
    { unsupported: { span: at("<details>"), construct: "html", source: "<details>raw HTML</details>\n" } },
    { footnote: { span: at("[^1]:"), name: "1", blocks: [{ paragraph: { span: at("The closure"), inline: [words("The closure rule is D31.")] } }] } },
  ];

  // wording-ok: fixture text is a model's reply, not a page's words
  const CLOSED = ["- `crates/kernel`", "- **Ledger**", "", "| step | what |", "|---|---|", "| 1 | read |", "", "```rust", "fn main() {}", "```", "", ""].join("\n");
  const STREAMING = `${CLOSED}then the plan it asks for, one step per`;
  const by = (source: string) => spanIn(CLOSED, source);
  const CLOSED_BLOCKS: readonly Block[] = [
    {
      list: {
        span: by("- `crates"),
        order: "bullet",
        spacing: "tight",
        items: [
          { span: by("- `crates"), check: "not_a_task", blocks: [{ paragraph: { span: by("`crates"), inline: [{ code: "crates/kernel" }] } }] },
          { span: by("- **"), check: "not_a_task", blocks: [{ paragraph: { span: by("**Ledger**"), inline: [{ strong: [words("Ledger")] }] } }] },
        ],
      },
    },
    {
      table: {
        span: by("| step"),
        align: ["none", "none"],
        head: { cells: [[words("step")], [words("what")]] },
        body: [{ cells: [[words("1")], [words("read")]] }],
      },
    },
    { code: { span: by("```rust"), info: "rust", text: "fn main() {}\n" } },
  ];

  // wording-ok: fixture text is a model's reply, not a page's words
  const FENCED_HEAD = "Before the code.\n\n";
  const FENCED = `${FENCED_HEAD}\`\`\`rust\nfn a() {}\n\nfn b`;

  // wording-ok: fixture text is a model's reply, not a page's words
  const REFUSED = "A reply holding a NUL\u0000 is not text the city reads.\n\nIt is drawn as it was said.";
  // wording-ok: fixture text is a model's reply, not a page's words
  const UNANSWERED = "## Not answered yet\n\nUntil the city lays it out, every word is drawn as it was said.";

  const laid = (text: string, blocks: readonly Block[]): Answer => ({ reply: { span: { start: 0, end: bytes(text) }, blocks } });

  // Each question the cases below ask, by the text it carries.
  const REPLIES: Readonly<Record<string, Answer>> = {
    [SETTLED]: laid(SETTLED, SETTLED_BLOCKS),
    [CLOSED]: laid(CLOSED, CLOSED_BLOCKS),
    [`${FENCED_HEAD}\`\`\`rust\nfn a() {}\n\n`]: laid(FENCED_HEAD, [{ paragraph: { span: spanIn(FENCED, "Before"), inline: [words("Before the code.")] } }]),
    [REFUSED]: { unavailable: { query: "Reply" } },
  };

  function answering(query: Query): Answer | undefined {
    return typeof query === "object" && "reply" in query ? REPLIES[query.reply.text] : undefined;
  }
</script>

<script lang="ts">
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import Prose from "../prose.svelte";
  import Saying from "../talk/saying.svelte";
  import Case from "./case.svelte";
  import Stand from "./stand.svelte";

  // The conversation column's width, which is where a reply is read.
  const TALK = 760;

  const { lang } = ui();
  const who = $derived(say($lang, "talk_resident"));
</script>

<Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answering}>
  <Case label="reply · settled, every block the city lays out" width={TALK}>
    <div class="max-w-[64ch] text-body"><Prose text={SETTLED} /></div>
  </Case>
  <Case label="reply · streaming, the closed blocks laid out and the open paragraph raw" width={TALK}>
    <Saying text={STREAMING} {who} />
  </Case>
  <Case label="reply · streaming inside an open code fence, raw until it closes" width={TALK}>
    <Saying text={FENCED} {who} />
  </Case>
  <Case label="reply · a text the city will not read, drawn as it was said" width={TALK}>
    <div class="max-w-[64ch] text-body"><Prose text={REFUSED} /></div>
  </Case>
  <Case label="reply · before the city answers, every word as it was said" width={TALK}>
    <div class="max-w-[64ch] text-body"><Prose text={UNANSWERED} /></div>
  </Case>
</Stand>

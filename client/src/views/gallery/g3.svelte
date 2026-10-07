<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // Documents in the content store (client/Spec.lean §4-61), on a made-up city
  // that answers `Versions`, `Range` and `Bytes` the way a served city
  // does: every version the city knows of a Markdown file, one of them
  // written outside the page and never kept; a PDF in a building drawn
  // from bytes fetched window by window; and a screenshot drawn from the
  // store. Each is drawn at the width the right side has beside a
  // conversation at 1440.
  import { Address, B3Hash, Locator, Seq, TimeMs } from "../../wire";
  import type { Answer, DocumentState, Query } from "../../wire";
  import { pdfOf, utf8 } from "./fmt_samples";

  const SHOP = Address.make("shop");
  const MEMO = "notes/memo.md";
  const REPORT = "out/report.pdf";

  const V0 = B3Hash.make("1d2c3b4a".repeat(8));
  const V1 = B3Hash.make("5e6f7a8b".repeat(8));
  const V2 = B3Hash.make("9c0d1e2f".repeat(8));
  const V3 = B3Hash.make("a3b4c5d6".repeat(8));
  const PDF = B3Hash.make("e7f80912".repeat(8));
  const SHOT = B3Hash.make("c3".repeat(32));

  // wording-ok: fixture text is a file's contents, not a page's words
  const TEXTS: Readonly<Record<string, string>> = {
    [V0]: "# Memo\n\nThe first draft.\n",
    [V1]: "# Memo\n\nThe first draft, read twice.\n",
    [V3]: "# Memo\n\nThe first draft, read twice.\n\nA resident added this line.\n",
  };

  // wording-ok: fixture text is a file's contents, not a page's words
  const REPORT_BYTES = pdfOf([
    { latin: ["Quarterly report", "Every version the city reads is kept."], chinese: ["城读到的每一版都存下。"] },
  ]);

  // A picture of the page the browser tool looked at: a drawing rather
  // than a photograph, so the fixture carries no image file. Its fills
  // are the theme's roles, read off the page when the picture is asked
  // for, so the drawing holds no colour of its own and follows the
  // lighting the gallery is drawn in.
  const shotBytes = (): Uint8Array => {
    const drawn = getComputedStyle(document.documentElement);
    const fill = (role: string): string => drawn.getPropertyValue(`--color-${role}`).trim();
    return utf8(
      [
        '<svg xmlns="http://www.w3.org/2000/svg" width="1280" height="720" viewBox="0 0 1280 720">',
        `<rect width="1280" height="720" fill="${fill("page")}"/>`,
        `<rect x="0" y="0" width="1280" height="64" fill="${fill("chrome")}"/>`,
        `<rect x="80" y="120" width="560" height="40" rx="6" fill="${fill("raised-hover")}"/>`,
        `<rect x="80" y="190" width="1120" height="16" rx="4" fill="${fill("track")}"/>`,
        `<rect x="80" y="222" width="980" height="16" rx="4" fill="${fill("track")}"/>`,
        `<rect x="80" y="290" width="520" height="300" rx="10" fill="${fill("accent")}"/>`,
        `<rect x="680" y="290" width="520" height="300" rx="10" fill="${fill("mark")}"/>`,
        "</svg>",
      ].join(""),
    );
  };

  const bytesOf = (text: string): number => utf8(text).length;
  const at = (seq: number, ms: number): { saved: { seq: Seq; at: TimeMs } } => ({
    saved: { seq: Seq.make(seq), at: TimeMs.make(ms) },
  });

  const STATES: Readonly<Record<string, DocumentState>> = {
    [`shop/${MEMO}`]: {
      held: {
        version: V3,
        format: "markdown",
        bytes: bytesOf(TEXTS[V3] ?? ""),
        body: {
          text: {
            encoding: "utf8",
            coverage: "whole",
            head: { span: { start: 0, end: bytesOf(TEXTS[V3] ?? "") }, text: TEXTS[V3] ?? "" },
          },
        },
      },
    },
  };

  // At most this many bytes a window, so the PDF arrives in several.
  const WINDOW = 1024;

  function base64Of(bytes: Uint8Array): string {
    return btoa(Array.from(bytes, (byte) => String.fromCharCode(byte)).join(""));
  }

  function answering(query: Query): Answer | undefined {
    if (typeof query !== "object") return undefined;
    if ("document" in query) return { document: { at: query.document.at, state: STATES[query.document.at] ?? "missing" } };
    if ("versions" in query) {
      return {
        versions: {
          at: query.versions.at,
          versions: [
            { version: V3, bytes: bytesOf(TEXTS[V3] ?? ""), kept: true, source: "on_disk" },
            { version: V2, kept: false, source: { before: { seq: Seq.make(412) } } },
            { version: V1, bytes: bytesOf(TEXTS[V1] ?? ""), kept: true, source: at(388, 1_790_000_000_000) },
            { version: V0, bytes: bytesOf(TEXTS[V0] ?? ""), kept: true, source: { before: { seq: Seq.make(388) } } },
          ],
          more: false,
        },
      };
    }
    if ("range" in query) {
      const text = TEXTS[query.range.version] ?? "";
      const start = query.range.range.start;
      const window = start === 0 ? { span: { start: 0, end: bytesOf(text) }, text } : { span: { start, end: start }, text: "" };
      return { range: { version: query.range.version, window } };
    }
    if ("bytes" in query) {
      const whole = query.bytes.version === PDF ? REPORT_BYTES : query.bytes.version === SHOT ? shotBytes() : null;
      if (whole === null) return { unavailable: { query: "Bytes" } };
      const start = Math.min(query.bytes.range.start, whole.length);
      const end = Math.min(whole.length, start + WINDOW, query.bytes.range.end);
      return { bytes: { version: query.bytes.version, span: { start, end }, size: whole.length, base64: base64Of(whole.slice(start, end)) } };
    }
    return undefined;
  }

  const PICTURE = { image: Locator.make(`cas:b3-${SHOT}`), width: 1280, height: 720, media_type: "image/svg+xml" };
</script>

<script lang="ts">
  import Shot from "../inspect/shot.svelte";
  import Document from "../refrain/document.svelte";
  import Opaque from "../refrain/formats/opaque.svelte";
  import Case from "./case.svelte";
  import Stand from "./stand.svelte";

  const FRAME = "refrain-specimen flex h-[480px] flex-col overflow-hidden bg-page";
</script>

<Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answering}>
  <Case label="refrain · every version the city knows, one written outside the page and not kept" width={600}>
    <div class={FRAME}>
      <Document at={Address.make(`shop/${MEMO}`)} building={SHOP} version={null} reading="versions" />
    </div>
  </Case>
  <Case label="refrain · a city's PDF drawn from bytes fetched by its version" width={600}>
    <div class={FRAME}>
      <!-- The part itself rather than the document around it: a part a
      document mounts once its answer lands mounts after the stand has
      handed `ui.ts`'s door back to the real city, and would ask that
      city for bytes this one holds. -->
      <Opaque path={Address.make(`shop/${REPORT}`)} version={PDF} bytes={REPORT_BYTES.length} />
    </div>
  </Case>
  <Case label="inspect · a screenshot drawn from the content store" width={600}>
    <div class="flex h-[420px] flex-col overflow-hidden bg-page"><Shot picture={PICTURE} /></div>
  </Case>
</Stand>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { GalleryCase } from "./case";
import Stand from "./stand.solid";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import { againstLookOf } from "../editor/formats/against";
import Against from "../editor/formats/against.look.solid";
import Compare from "../editor/formats/compare_view";
import Docx from "../editor/formats/docx_view";
import RefRain, { DocumentPage } from "../editor/document_view";
import { PdfCase } from "./fmt_pdf";

import { Address, B3Hash } from "../../wire";
import type { Answer, DocumentState, Query } from "../../wire";
import { UNPACKED_MAX } from "../editor/formats/zip";
import { docxOf, pdfOf, utf8, zipOf } from "./fmt_samples";

const SHOP = Address.make("shop");
const PAGE = "site/index.html";
const REPORT = "reports/brief.pdf";

const V1 = B3Hash.make("6a0c3e91".repeat(8));
const V2 = B3Hash.make("b4d27f08".repeat(8));
const V3 = B3Hash.make("1e9a54c7".repeat(8));

const BRIEF = {
  latin: [
    "Quarterly brief",
    "Revenue rose in every region but one.",
    "The north fell behind its plan by six weeks.",
  ],
  chinese: [],
};
const BRIEF_NEXT = {
  ...BRIEF,
  latin: [
    "Quarterly brief",
    "Revenue rose in every region.",
    "The north fell behind its plan by six weeks.",
  ],
};
const NOTES = { latin: ["Notes"], chinese: ["中文排版：标点挤压与行首禁则。"] };
const PDF_V1 = pdfOf([BRIEF, NOTES]);
const PDF_V2 = pdfOf([BRIEF_NEXT, NOTES]);

const DOCX_V1 = docxOf([
  { text: "Release plan" },
  { text: "The client ships with the city." },
  { text: "Every preview names its tool and version." },
]);
const DOCX_V2 = docxOf([
  { text: "Release plan" },
  { text: "The client ships with the city", inserted: " and its fonts." },
  { text: "Every preview names its tool, its settings and its version." },
]);
const DOCX_LARGE = zipOf([
  { name: "word/document.xml", bytes: utf8("<w:document/>"), stated: UNPACKED_MAX + 1 },
]);

const HTML = [
  "<!doctype html>",
  '<html lang="en"><head><meta charset="utf-8"><title>Shop</title>',
  "<style>body { font: 16px/1.5 Georgia, serif; margin: 32px; } h1 { font-size: 28px; } .note { padding: 8px 12px; border-left: 3px solid; }</style>",
  '<script>document.body.textContent = "a script ran";</' + "script>",
  "</head><body>",
  "<h1>Opening hours</h1>",
  "<p>Monday to Friday, nine to six. Closed on public holidays.</p>",
  '<p class="note">The picture below is on another server, so the preview does not fetch it.</p>',
  '<img src="https://example.com/storefront.jpg" alt="storefront" width="240" height="120">',
  '<p><a href="https://example.com/map" target="_top">Map</a></p>',
  "</body></html>",
  "",
].join("\n");

const STATES: Readonly<Record<string, DocumentState>> = {
  [`shop/${PAGE}`]: {
    held: {
      version: V1,
      format: "plain",
      bytes: utf8(HTML).length,
      body: {
        text: {
          encoding: "utf8",
          coverage: "whole",
          head: { span: { start: 0, end: utf8(HTML).length }, text: HTML },
        },
      },
    },
  },
  [`shop/${REPORT}`]: {
    held: { version: V3, format: "plain", bytes: PDF_V2.length, body: "opaque" },
  },
};

function answering(query: Query): Answer | undefined {
  if (typeof query !== "object" || !("document" in query)) return undefined;
  const at = query.document.at;
  return { document: { at, state: STATES[at] ?? "missing" } };
}
const EARLIER = [
  { version: V1, bytes: PDF_V1.length, kept: true, source: "on_disk" },
  { version: V3, bytes: PDF_V2.length, kept: true, source: "on_disk" },
] as const;
const ignore = (): void => undefined;

const FRAME = "refrain flex h-[480px] flex-col overflow-hidden bg-page";

export default function FormatCases() {
  const lang = storeValue(ui().lang);
  return (
    <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answering}>
      {() => (
        <>
          <PdfCase bytes={PDF_V2} version={V2} frame={FRAME} lang={lang()} />
          <GalleryCase
            label="formats · a DOCX laid out as pages, its tracked change and comment counted"
            width={600}
          >
            <div class={FRAME}>
              <Docx bytes={DOCX_V2} version={V2} name="plan.docx" />
            </div>
          </GalleryCase>
          <GalleryCase
            label="formats · an HTML page in RefRain's preview, its script and outside picture held back"
            width={600}
          >
            <div class={FRAME}>
              <DocumentPage
                at={Address.make(`shop/${PAGE}`)}
                building={SHOP}
                version={null}
                reading="preview"
              />
            </div>
          </GalleryCase>
          <GalleryCase
            label="formats · two versions of a PDF compared by the text of their pages"
            width={600}
          >
            <div class={FRAME}>
              <Compare
                format="pdf"
                from={{ version: V1, bytes: PDF_V1 }}
                to={{ version: V2, bytes: PDF_V2 }}
                name="brief.pdf"
              />
            </div>
          </GalleryCase>
          <GalleryCase
            label="formats · two versions of a DOCX compared by their paragraphs"
            width={600}
          >
            <div class={FRAME}>
              <Compare
                format="docx"
                from={{ version: V1, bytes: DOCX_V1 }}
                to={{ version: V2, bytes: DOCX_V2 }}
                name="plan.docx"
              />
            </div>
          </GalleryCase>
          <GalleryCase
            label="formats · the version picker above a PDF, compared with nothing"
            width={600}
          >
            <Against {...againstLookOf(EARLIER, null, lang(), ignore)} />
          </GalleryCase>
          <GalleryCase
            label="formats · the version picker above a PDF, compared with an earlier version"
            width={600}
          >
            <Against {...againstLookOf(EARLIER, V1, lang(), ignore)} />
          </GalleryCase>
          <GalleryCase
            label="formats · a DOCX that unpacks past the bound, refused unopened"
            width={600}
          >
            <div class={FRAME}>
              <Docx bytes={DOCX_LARGE} version={V3} name="large.docx" />
            </div>
          </GalleryCase>
          <GalleryCase
            label="formats · a PDF RefRain opens from the city, which sends no bytes"
            width={600}
          >
            <div class={FRAME}>
              <RefRain building={SHOP} path={REPORT} version={null} />
            </div>
          </GalleryCase>
        </>
      )}
    </Stand>
  );
}

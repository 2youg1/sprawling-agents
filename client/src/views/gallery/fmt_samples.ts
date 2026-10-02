// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The files the format fixtures draw, written out byte by byte here
// rather than kept as binary files, so a reader sees exactly what each
// specimen holds: a two-page PDF whose second page sets Chinese in a
// font the file leaves out (so its CMap is fetched), and a DOCX with a
// tracked change and a comment. `zip.test.ts` builds its archives with
// the same writer.

export interface Entry {
  readonly name: string;
  readonly bytes: Uint8Array;
}

const encoder = new TextEncoder();

export function utf8(text: string): Uint8Array {
  return encoder.encode(text);
}

// A ZIP archive of `entries`, each stored, or deflated when `packed`
// gives its deflated bytes. `stated` overrides the size the directory
// states for a part, which is how a test writes an archive that lies.
export function zipOf(
  entries: readonly (Entry & { readonly packed?: Uint8Array; readonly stated?: number })[],
): Uint8Array {
  const locals: Uint8Array[] = [];
  const directory: Uint8Array[] = [];
  let offset = 0;
  for (const entry of entries) {
    const name = utf8(entry.name);
    const data = entry.packed ?? entry.bytes;
    const method = entry.packed === undefined ? 0 : 8;
    const size = entry.stated ?? entry.bytes.length;
    const crc = crc32(entry.bytes);
    const local = record(30 + name.length, (view) => {
      view.setUint32(0, 0x04034b50, true);
      view.setUint16(4, 20, true);
      view.setUint16(8, method, true);
      view.setUint32(14, crc, true);
      view.setUint32(18, data.length, true);
      view.setUint32(22, size, true);
      view.setUint16(26, name.length, true);
    });
    local.set(name, 30);
    const central = record(46 + name.length, (view) => {
      view.setUint32(0, 0x02014b50, true);
      view.setUint16(4, 20, true);
      view.setUint16(6, 20, true);
      view.setUint16(10, method, true);
      view.setUint32(16, crc, true);
      view.setUint32(20, data.length, true);
      view.setUint32(24, size, true);
      view.setUint16(28, name.length, true);
      view.setUint32(42, offset, true);
    });
    central.set(name, 46);
    locals.push(local, data);
    directory.push(central);
    offset += local.length + data.length;
  }
  const size = directory.reduce((sum, each) => sum + each.length, 0);
  const end = record(22, (view) => {
    view.setUint32(0, 0x06054b50, true);
    view.setUint16(8, entries.length, true);
    view.setUint16(10, entries.length, true);
    view.setUint32(12, size, true);
    view.setUint32(16, offset, true);
  });
  return joined([...locals, ...directory, end]);
}

export function deflated(bytes: Uint8Array): Promise<Uint8Array> {
  const stream = new Blob([bytes.slice()]).stream().pipeThrough(new CompressionStream("deflate-raw"));
  return new Response(stream).arrayBuffer().then((buffer) => new Uint8Array(buffer));
}

function record(length: number, write: (view: DataView) => void): Uint8Array {
  const bytes = new Uint8Array(length);
  write(new DataView(bytes.buffer));
  return bytes;
}

function joined(parts: readonly Uint8Array[]): Uint8Array {
  const out = new Uint8Array(parts.reduce((sum, each) => sum + each.length, 0));
  let at = 0;
  for (const part of parts) {
    out.set(part, at);
    at += part.length;
  }
  return out;
}

function crc32(bytes: Uint8Array): number {
  let crc = 0xffffffff;
  for (const byte of bytes) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit += 1) crc = crc & 1 ? (crc >>> 1) ^ 0xedb88320 : crc >>> 1;
  }
  return (crc ^ 0xffffffff) >>> 0;
}

// ------------------------------------------------------------------ PDF

// One page: lines of Latin text in Helvetica, which the file does not
// embed, and lines of Chinese in STSong-Light through the UniGB-UCS2-H
// CMap, which pdf.js fetches from the bundle.
export interface Page {
  readonly latin: readonly string[];
  readonly chinese: readonly string[];
}

export function pdfOf(pages: readonly Page[]): Uint8Array {
  const kids = pages.map((_, index) => `${String(6 + index * 2)} 0 R`).join(" ");
  const objects = [
    "<< /Type /Catalog /Pages 2 0 R >>",
    `<< /Type /Pages /Kids [${kids}] /Count ${String(pages.length)} >>`,
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    "<< /Type /Font /Subtype /Type0 /BaseFont /STSong-Light /Encoding /UniGB-UCS2-H /DescendantFonts [5 0 R] >>",
    "<< /Type /Font /Subtype /CIDFontType0 /BaseFont /STSong-Light /CIDSystemInfo << /Registry (Adobe) /Ordering (GB1) /Supplement 4 >> >>",
    ...pages.flatMap((page, index) => {
      const content = contentOf(page);
      return [
        `<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 3 0 R /F2 4 0 R >> >> /Contents ${String(7 + index * 2)} 0 R >>`,
        `<< /Length ${String(content.length)} >>\nstream\n${content}\nendstream`,
      ];
    }),
  ];
  let text = "%PDF-1.7\n";
  const offsets: number[] = [];
  objects.forEach((body, index) => {
    offsets.push(text.length);
    text += `${String(index + 1)} 0 obj\n${body}\nendobj\n`;
  });
  const xref = text.length;
  text += `xref\n0 ${String(objects.length + 1)}\n0000000000 65535 f \n`;
  text += offsets.map((at) => `${String(at).padStart(10, "0")} 00000 n \n`).join("");
  text += `trailer\n<< /Size ${String(objects.length + 1)} /Root 1 0 R >>\nstartxref\n${String(xref)}\n%%EOF\n`;
  return utf8(text);
}

function contentOf(page: Page): string {
  // A title above a rule, then the body lines under it.
  const latin = page.latin.map((line, index) => {
    const size = index === 0 ? 22 : 12;
    const y = index === 0 ? 760 : 724 - (index - 1) * 22;
    return `BT /F1 ${String(size)} Tf 64 ${String(y)} Td (${escaped(line)}) Tj ET`;
  });
  const chinese = page.chinese.map((line, index) => {
    // UCS-2: one four-digit code unit per character of the BMP.
    const hex = Array.from({ length: line.length }, (_, at) => line.charCodeAt(at).toString(16).padStart(4, "0")).join("");
    return `BT /F2 14 Tf 64 ${String(700 - index * 26)} Td <${hex}> Tj ET`;
  });
  return ["0.5 w 64 748 m 531 748 l S", ...latin, ...chinese].join("\n");
}

function escaped(line: string): string {
  return line.replaceAll("\\", "\\\\").replaceAll("(", "\\(").replaceAll(")", "\\)");
}

// ----------------------------------------------------------------- DOCX

const W = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

// A relationship's type URI, assembled once from its segments: written
// out whole, its path reads to `xtask secret` as one long token.
const RELATION = ["http://schemas.openxmlformats.org", "officeDocument", "2006", "relationships"].join("/");

// A DOCX with these paragraphs, the first a heading. A paragraph may
// carry one tracked insertion, and the first paragraph a comment, so the
// preview has a revision and a comment to account for.
export function docxOf(paragraphs: readonly { readonly text: string; readonly inserted?: string }[]): Uint8Array {
  const body = paragraphs
    .map((each, index) => {
      const style = index === 0 ? '<w:pPr><w:pStyle w:val="Title"/></w:pPr>' : "";
      const remark = index === 0 ? '<w:commentRangeStart w:id="0"/>' : "";
      const after = index === 0 ? '<w:commentRangeEnd w:id="0"/><w:r><w:commentReference w:id="0"/></w:r>' : "";
      const inserted =
        each.inserted === undefined
          ? ""
          : `<w:ins w:id="${String(index + 1)}" w:author="resident" w:date="2026-01-01T00:00:00Z"><w:r><w:t xml:space="preserve">${each.inserted}</w:t></w:r></w:ins>`;
      return `<w:p>${style}${remark}<w:r><w:t xml:space="preserve">${each.text}</w:t></w:r>${inserted}${after}</w:p>`;
    })
    .join("");
  const parts: readonly Entry[] = [
    {
      name: "[Content_Types].xml",
      bytes: utf8(
        '<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/comments.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.comments+xml"/></Types>',
      ),
    },
    {
      name: "_rels/.rels",
      bytes: utf8(
        `<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="${RELATION}/officeDocument" Target="word/document.xml"/></Relationships>`,
      ),
    },
    {
      name: "word/_rels/document.xml.rels",
      bytes: utf8(
        `<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="${RELATION}/styles" Target="styles.xml"/><Relationship Id="rId2" Type="${RELATION}/comments" Target="comments.xml"/></Relationships>`,
      ),
    },
    {
      name: "word/document.xml",
      bytes: utf8(
        `<?xml version="1.0" encoding="UTF-8"?><w:document xmlns:w="${W}"><w:body>${body}<w:sectPr><w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1134" w:right="1134" w:bottom="1134" w:left="1134"/></w:sectPr></w:body></w:document>`,
      ),
    },
    {
      name: "word/styles.xml",
      bytes: utf8(
        `<?xml version="1.0" encoding="UTF-8"?><w:styles xmlns:w="${W}"><w:style w:type="paragraph" w:styleId="Title"><w:name w:val="Title"/><w:rPr><w:b/><w:sz w:val="40"/></w:rPr></w:style></w:styles>`,
      ),
    },
    {
      name: "word/comments.xml",
      bytes: utf8(
        `<?xml version="1.0" encoding="UTF-8"?><w:comments xmlns:w="${W}"><w:comment w:id="0" w:author="resident"><w:p><w:r><w:t>Check the figures.</w:t></w:r></w:p></w:comment></w:comments>`,
      ),
    },
  ];
  return zipOf(parts);
}

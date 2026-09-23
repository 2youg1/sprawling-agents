// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The colouring knows words, never structure. It walks a line and names
// what it passes - a comment, a string, a number, one of the words the
// family below lists - and it never builds a tree. A file whose
// extension is not in the table is drawn in one ink, which is the honest
// reading of "this build cannot tell". A tokenizer dependency would buy
// the difference for about two hundred kilobytes of a bundle that exists
// to show a few dozen lines at a time.

// What a stretch of a line is, as far as a reader without a grammar can
// tell. Four classes and a fifth for everything else, because the theme
// offers four inks a reader can hold apart at note size.
export type Ink = "plain" | "comment" | "string" | "number" | "word";

export interface Piece {
  readonly ink: Ink;
  readonly text: string;
}

// What one family of files is made of, lexically. Not a language
// definition: it is the shortest description under which a comment, a
// string and a keyword can be told apart on one line.
interface Syntax {
  // What opens a comment that runs to the end of the line.
  readonly line: string;
  // What opens and closes a comment that may cross lines, when the
  // family has one.
  readonly block: readonly [string, string] | null;
  // Every character that both opens and closes a string. A backslash
  // inside one escapes the character after it.
  readonly quotes: readonly string[];
  // The words drawn as words. Short on purpose: a reader scanning a
  // diff wants the shape of the line, and a complete keyword list for
  // six languages would be a table nobody maintains.
  readonly words: readonly string[];
}

const BRACES: Syntax = {
  line: "//",
  block: ["/*", "*/"],
  quotes: ['"', "'", "`"],
  words: [
    "async", "await", "break", "case", "const", "continue", "else", "enum",
    "export", "fn", "for", "func", "function", "if", "impl", "import",
    "interface", "let", "match", "mut", "pub", "return", "struct", "switch",
    "trait", "type", "use", "var", "while",
  ],
};

const HASHES: Syntax = {
  line: "#",
  block: null,
  quotes: ['"', "'"],
  words: [
    "async", "await", "break", "case", "class", "def", "elif", "else",
    "esac", "fi", "for", "from", "if", "import", "in", "lambda", "return",
    "then", "while", "with",
  ],
};

// Which family a file belongs to, by the last part of its name. A name
// this table does not hold is drawn without colour rather than guessed
// at, because a wrong comment marker hides a line instead of dimming
// it.
const SYNTAX = new Map<string, Syntax>([
  ["c", BRACES],
  ["cpp", BRACES],
  ["go", BRACES],
  ["h", BRACES],
  ["java", BRACES],
  ["js", BRACES],
  ["json", BRACES],
  ["jsx", BRACES],
  ["rs", BRACES],
  ["ts", BRACES],
  ["tsx", BRACES],
  ["zig", BRACES],
  ["bash", HASHES],
  ["py", HASHES],
  ["sh", HASHES],
  ["toml", HASHES],
  ["yaml", HASHES],
  ["yml", HASHES],
]);

const WORD_START = /[A-Za-z_$]/;
const WORD_PART = /[A-Za-z0-9_$]/;
const DIGIT = /[0-9]/;

// Where the file's name stops and its family begins. A name with no
// dot, or a dotfile whose only dot starts it, has no extension.
function extensionOf(path: string): string {
  const name = path.slice(path.lastIndexOf("/") + 1);
  const dot = name.lastIndexOf(".");
  return dot <= 0 ? "" : name.slice(dot + 1).toLowerCase();
}

// Where the string opened at `from` ends, one past its closing quote,
// or the end of the line when it does not close there. A string that
// runs past a line end is coloured on the line it opened on and no
// further: carrying that state would make a stray quote recolour the
// rest of a file, which is louder than the mistake it reports.
function closes(line: string, from: number, quote: string): number {
  let at = from + 1;
  while (at < line.length) {
    const here = line.charAt(at);
    if (here === "\\") {
      at += 2;
      continue;
    }
    if (here === quote) return at + 1;
    at += 1;
  }
  return line.length;
}

interface Scanned {
  readonly pieces: readonly Piece[];
  // Whether a block comment is still open when this line ends.
  readonly open: boolean;
}

function scan(line: string, syntax: Syntax, opened: boolean): Scanned {
  const pieces: Piece[] = [];
  let plain = "";
  let at = 0;
  let open = opened;
  const keep = (ink: Ink, text: string) => {
    if (plain !== "") {
      pieces.push({ ink: "plain", text: plain });
      plain = "";
    }
    pieces.push({ ink, text });
  };
  while (at < line.length) {
    const block = syntax.block;
    if (open && block !== null) {
      const ends = line.indexOf(block[1], at);
      const upto = ends === -1 ? line.length : ends + block[1].length;
      keep("comment", line.slice(at, upto));
      open = ends === -1;
      at = upto;
      continue;
    }
    if (block !== null && line.startsWith(block[0], at)) {
      open = true;
      continue;
    }
    if (line.startsWith(syntax.line, at)) {
      keep("comment", line.slice(at));
      at = line.length;
      continue;
    }
    const here = line.charAt(at);
    if (syntax.quotes.includes(here)) {
      const upto = closes(line, at, here);
      keep("string", line.slice(at, upto));
      at = upto;
      continue;
    }
    if (DIGIT.test(here) || WORD_START.test(here)) {
      let upto = at;
      while (upto < line.length && WORD_PART.test(line.charAt(upto))) upto += 1;
      const run = line.slice(at, upto);
      if (DIGIT.test(here)) {
        keep("number", run);
      } else if (syntax.words.includes(run)) {
        keep("word", run);
      } else {
        plain += run;
      }
      at = upto;
      continue;
    }
    plain += here;
    at += 1;
  }
  if (plain !== "") pieces.push({ ink: "plain", text: plain });
  return { pieces, open };
}

// The whole artifact as one run of pieces, newlines included, so the
// column that draws it is a flat list rather than a box per line. The
// path chooses the family; a name the table above does not hold is one
// run of plain ink.
export function painted(text: string, path: string): readonly Piece[] {
  const syntax = SYNTAX.get(extensionOf(path)) ?? null;
  if (syntax === null) return [{ ink: "plain", text }];
  const pieces: Piece[] = [];
  let open = false;
  const lines = text.split("\n");
  for (const [at, line] of lines.entries()) {
    const said = scan(line, syntax, open);
    pieces.push(...said.pieces);
    open = said.open;
    if (at < lines.length - 1) pieces.push({ ink: "plain", text: "\n" });
  }
  return pieces;
}

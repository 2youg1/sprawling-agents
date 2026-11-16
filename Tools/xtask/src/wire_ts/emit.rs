// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use serde_json::Value;

use super::Constants;
use types::{Side, Types, object_body};
use values::Values;

mod types;
mod values;

#[derive(Debug)]
pub(super) struct Refused {
    pub(super) at: String,
    pub(super) why: String,
}

#[derive(Debug)]
pub(super) struct Bundle {
    pub(super) files: Vec<(String, String)>,
}

impl Bundle {
    #[cfg(test)]
    pub(super) fn text(&self) -> String {
        self.files
            .iter()
            .map(|(_, body)| body.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

const REF_PREFIX: &str = "#/$defs/";
const CHUNK_BODY_LIMIT: usize = 400;
const WIRE_DIR: &str = "wire";

pub(super) fn emit(document: &Value, constants: &Constants) -> Result<Bundle, Refused> {
    let defs: BTreeMap<&str, &Value> = document
        .get("$defs")
        .and_then(Value::as_object)
        .ok_or_else(|| refuse("$defs", "the document has no `$defs` object"))?
        .iter()
        .map(|(name, schema)| (name.as_str(), schema))
        .collect();
    let edges = edges(&defs);
    let recursive = recursive(&edges);
    let mut bodies: Vec<(BTreeSet<String>, String)> = Vec::new();
    let mut current_names: BTreeSet<String> = BTreeSet::new();
    let mut current_body = String::new();
    let mut current_lines = 0_usize;
    for name in ordered(&edges)? {
        let schema = defs
            .get(name.as_str())
            .ok_or_else(|| refuse(&name, "named by a `$ref` and absent from `$defs`"))?;
        let piece = definition(&name, schema, &recursive)?;
        let piece_lines = piece.lines().count();
        if current_lines > 0 && current_lines.saturating_add(piece_lines) > CHUNK_BODY_LIMIT {
            bodies.push((
                std::mem::take(&mut current_names),
                std::mem::take(&mut current_body),
            ));
            current_lines = 0;
        }
        current_names.insert(name);
        current_body.push_str(&piece);
        current_lines = current_lines.saturating_add(piece_lines);
    }
    if current_lines > 0 || bodies.is_empty() {
        bodies.push((current_names, current_body));
    }

    let mut defined: Vec<(usize, BTreeSet<String>)> = Vec::new();
    let mut chunks: Vec<String> = Vec::with_capacity(bodies.len());
    for (i, (names, body)) in bodies.into_iter().enumerate() {
        let mut header = String::from(CHUNK_HEADER);
        let mut needed: BTreeMap<usize, BTreeSet<String>> = BTreeMap::new();
        for (earlier, earlier_names) in &defined {
            for name in earlier_names {
                if body_mentions(&body, name) {
                    needed.entry(*earlier).or_default().insert(name.clone());
                }
            }
        }
        for (earlier, imports) in needed {
            let _ = write!(header, "import {{\n");
            for name in &imports {
                let _ = writeln!(header, "  {name},");
            }
            let _ = writeln!(header, "}} from \"./c{earlier:02}\";\n");
        }
        header.push_str(&body);
        defined.push((i, names));
        chunks.push(header);
    }

    let mut index = String::from(INDEX_HEADER);
    let _ = write!(
        index,
        "export const WIRE_V = {} as const;\n\
         export const WIRE_HASH = \"{}\" as const;\n\
         export const CITY_RUN = \"{}\" as const;\n\
         export const BODY_PX = {{ min: {} }} as const;\n\
         export const LINE_HEIGHT = {{ min: {} }} as const;\n\
         export const TRACKING = {{ min: {} }} as const;\n\
         export const DEFAULT_EFFORT = {} as const;\n\n",
        constants.wire_v,
        constants.hash,
        constants.city_run,
        constants.body_px_min,
        constants.line_height_min,
        constants.tracking_min,
        quoted(constants.default_effort)
    );
    index.push_str("export const SLASH = [\n");
    for [spelling, takes, offered, says] in &constants.slash {
        let _ = writeln!(
            index,
            "  {{ spelling: {}, takes: {}, offered: {}, says: {} }},",
            quoted(spelling),
            quoted(takes),
            quoted(offered),
            quoted(says)
        );
    }
    index.push_str("] as const;\n\n");
    let mut files = Vec::with_capacity(chunks.len().saturating_add(1));
    for (i, chunk) in chunks.iter().enumerate() {
        let name = format!("c{i:02}");
        let _ = writeln!(index, "export * from \"./{name}\";");
        let _ = writeln!(index, "export type * from \"./{name}\";");
        files.push((format!("{WIRE_DIR}/{name}.ts"), chunk.clone()));
    }
    drop(defined);
    files.insert(0, (format!("{WIRE_DIR}/index.ts"), index));
    Ok(Bundle { files })
}

const INDEX_HEADER: &str = "// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

";

const CHUNK_HEADER: &str = "// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Schema } from \"effect\";

";

fn refuse(at: &str, why: impl Into<String>) -> Refused {
    Refused {
        at: at.to_owned(),
        why: why.into(),
    }
}

fn body_mentions(body: &str, name: &str) -> bool {
    let mut rest = body;
    while let Some(at) = rest.find(name) {
        let before = rest[..at].chars().next_back();
        let after = rest[at.saturating_add(name.len())..].chars().next();
        let start_ok = before.is_none_or(|c| !c.is_ascii_alphanumeric() && c != '_');
        let end_ok = after.is_none_or(|c| !c.is_ascii_alphanumeric() && c != '_');
        if start_ok && end_ok {
            return true;
        }
        rest = &rest[at.saturating_add(1)..];
    }
    false
}

fn ordered(edges: &BTreeMap<String, BTreeSet<String>>) -> Result<Vec<String>, Refused> {
    let mut done = Vec::new();
    let mut trail = Vec::new();
    for name in edges.keys() {
        visit(name, edges, &mut trail, &mut done)?;
    }
    Ok(done)
}

fn visit(
    name: &str,
    edges: &BTreeMap<String, BTreeSet<String>>,
    trail: &mut Vec<String>,
    done: &mut Vec<String>,
) -> Result<(), Refused> {
    if done.iter().any(|d| d == name) || trail.iter().any(|t| t == name) {
        return Ok(());
    }
    let refs = edges
        .get(name)
        .ok_or_else(|| refuse(name, "named by a `$ref` and absent from `$defs`"))?;
    trail.push(name.to_owned());
    for target in refs {
        visit(target, edges, trail, done)?;
    }
    trail.pop();
    done.push(name.to_owned());
    Ok(())
}

fn edges(defs: &BTreeMap<&str, &Value>) -> BTreeMap<String, BTreeSet<String>> {
    defs.iter()
        .map(|(name, schema)| {
            let mut refs = BTreeSet::new();
            refs_within(schema, &mut refs);
            ((*name).to_owned(), refs)
        })
        .collect()
}

fn recursive(edges: &BTreeMap<String, BTreeSet<String>>) -> BTreeSet<String> {
    edges
        .keys()
        .filter(|name| {
            let mut seen = BTreeSet::new();
            let mut next: Vec<&String> = edges.get(*name).into_iter().flatten().collect();
            while let Some(at) = next.pop() {
                if at == *name {
                    return true;
                }
                if seen.insert(at) {
                    next.extend(edges.get(at).into_iter().flatten());
                }
            }
            false
        })
        .cloned()
        .collect()
}

fn refs_within(schema: &Value, out: &mut BTreeSet<String>) {
    match schema {
        Value::Object(map) => {
            if let Some(name) = map
                .get("$ref")
                .and_then(Value::as_str)
                .and_then(|target| target.strip_prefix(REF_PREFIX))
            {
                out.insert(name.to_owned());
            }
            map.values().for_each(|v| refs_within(v, out));
        }
        Value::Array(items) => items.iter().for_each(|v| refs_within(v, out)),
        _ => {}
    }
}

fn definition(name: &str, schema: &Value, recursive: &BTreeSet<String>) -> Result<String, Refused> {
    if !is_identifier(name) {
        return Err(refuse(name, "not a TypeScript identifier"));
    }
    let mut out = String::new();
    let none = BTreeSet::new();
    let is_recursive = recursive.contains(name);
    let suspended = if is_recursive { recursive } else { &none };
    let body = Values { suspended }.expression(schema, name, 0)?;

    let body = if is_bare_primitive(schema) {
        format!("{body}.pipe(Schema.brand(\"{name}\"))")
    } else {
        format!("{body}.annotate({{ identifier: \"{name}\" }})")
    };
    if is_recursive {
        out.push_str(&declared(name, Side::Type, schema, recursive)?);
        out.push_str(&declared(name, Side::Encoded, schema, recursive)?);
        let _ = writeln!(
            out,
            "export const {name}: Schema.Codec<{name}, {name}Encoded> = {body};\n"
        );
    } else {
        let _ = writeln!(out, "export const {name} = {body};");
        let _ = writeln!(out, "export type {name} = typeof {name}.Type;\n");
    }
    Ok(out)
}

fn declared(
    name: &str,
    side: Side,
    schema: &Value,
    recursive: &BTreeSet<String>,
) -> Result<String, Refused> {
    let spelled = match side {
        Side::Type => name.to_owned(),
        Side::Encoded => format!("{name}Encoded"),
    };
    let denoted = Types { recursive, side }.denoted(schema, name)?;
    Ok(match object_body(&denoted) {
        Some(fields) => format!("export interface {spelled} {{ {fields} }}\n"),
        None => format!("export type {spelled} = {denoted}\n"),
    })
}

fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn is_bare_primitive(schema: &Value) -> bool {
    let Some(map) = schema.as_object() else {
        return false;
    };
    matches!(
        map.get("type").and_then(Value::as_str),
        Some("string" | "integer" | "number" | "boolean")
    ) && !map.contains_key("enum")
        && !map.contains_key("const")
}

fn quoted(text: &str) -> String {
    Value::String(text.to_owned()).to_string()
}

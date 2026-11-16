// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use kernel::{AxCode, AxError, ReservedNamespace};
use serde_json::Value;

use super::{Catalog, Expansion, Filed, dormant};
use crate::tools::ToolTool;

pub const FIND_HITS: usize = 8;

pub const LIST_CEILING: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Grade {

    Text,

    Inside,

    Prefix,

    Exact,
}

impl Grade {
    fn weight(self) -> u32 {
        match self {
            Grade::Text => 1,
            Grade::Inside => 2,
            Grade::Prefix => 4,
            Grade::Exact => 8,
        }
    }
}

struct Candidate<'a> {
    namespace: &'a str,
    label: String,
    member: &'a str,
    about: &'a str,
    text: String,
}

impl Catalog {

    #[must_use]
    pub fn list(&self, namespace: Option<&str>) -> String {
        let Some(asked) = namespace.map(str::trim) else {
            let mut out: String = self
                .dormant_entries()
                .iter()
                .map(|entry| format!("- {}: {}\n", entry.label, dormant::hint(&entry.about)))
                .collect();
            if out.is_empty() {
                out.push_str("Nothing is admitted here beyond your tool list.\n");
            }
            return out;
        };
        let members: Vec<(String, &str)> = if asked == ReservedNamespace::Skill.as_str() {
            self.skills
                .iter()
                .map(|(name, admitted)| (name.clone(), admitted.entry.disclosure.as_str()))
                .collect()
        } else {
            self.tools
                .iter()
                .filter(|(_, filed)| filed.namespace.as_str() == asked)
                .map(|(name, filed)| (name.clone(), filed.def.description.as_str()))
                .collect()
        };
        if members.is_empty() {
            return format!(
                "No namespace here is called `{asked}`. `tool` `list` with no namespace names \
                 every one.\n"
            );
        }
        let hinted: String = members
            .iter()
            .map(|(name, about)| format!("- {name}: {}\n", dormant::hint(about)))
            .collect();
        if hinted.len() <= LIST_CEILING {
            return hinted;
        }
        members
            .iter()
            .map(|(name, _)| format!("- {name}\n"))
            .collect()
    }

    #[must_use]
    pub fn find(&self, query: &str) -> String {

        let words: Vec<String> = query
            .split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .filter(|word| !word.is_empty())
            .map(str::to_lowercase)
            .collect();
        let mut ranked: Vec<(u32, Candidate<'_>)> = self
            .candidates()
            .into_iter()
            .map(|candidate| (score(&words, &candidate), candidate))
            .filter(|(score, _)| *score > 0)
            .collect();
        ranked.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.label.cmp(&b.1.label)));
        if ranked.is_empty() {
            return format!(
                "Nothing admitted here matches `{}`. `tool` `list` names every namespace this \
                 building admits beyond your tool list.\n",
                query.trim()
            );
        }
        let mut grouped: BTreeMap<&str, Vec<String>> = BTreeMap::new();
        for (_, candidate) in ranked.iter().take(FIND_HITS) {
            grouped
                .entry(candidate.namespace)
                .or_default()
                .push(format!(
                    "- {}: {}\n",
                    candidate.label,
                    dormant::hint(candidate.about)
                ));
        }
        let mut out: String = grouped
            .into_iter()
            .map(|(namespace, lines)| format!("{namespace}:\n{}", lines.concat()))
            .collect();
        out.push_str("`tool` `detail` one by its whole name for its guide.\n");
        out
    }

    pub fn detail(&self, asked: &str) -> Result<String, AxError> {
        let asked = asked.trim();
        if let Some(filed) = self.tools.get(asked) {
            let schema = serde_json::to_string(&filed.def.input_schema).map_err(|err| {
                AxError::failure(AxCode::InvalidArgs, "detail a tool", asked.to_owned())
                    .with_recovery(format!(
                        "report this against runtime::catalog: a registered schema no \
                         longer prints as JSON ({err})"
                    ))
            })?;
            let how = if self.is_core(asked) {
                "it is in your tool list; call it directly".to_owned()
            } else {
                format!(
                    "run it with `{}` and the arguments {{\"op\": \"run\", \"name\": \
                     \"{asked}\", \"args\": {{...}}}}",
                    ToolTool::NAME
                )
            };
            return Ok(format!(
                "tool {asked} ({}): {how}.\n{}\ninput schema: {schema}\n",
                filed.namespace.as_str(),
                filed.def.description
            ));
        }

        let skill = asked.strip_prefix("skill ").unwrap_or(asked);
        if let Some(admitted) = self.skills.get(skill) {
            return Ok(format!(
                "skill {skill}: {}\nOpen its whole text with `read` and the path `{skill}`.\n",
                admitted.entry.disclosure
            ));
        }
        if let Some(Expansion::Said { text }) = self.expand(asked) {
            return Ok(text);
        }
        let words = asked.to_lowercase();
        Err(AxError::failure(
            AxCode::InvalidArgs,
            "detail a tool",
            format!("nothing admitted here is called `{asked}`"),
        )
        .with_nearby(
            self.candidates()
                .into_iter()
                .filter(|candidate| score(std::slice::from_ref(&words), candidate) > 1)
                .map(|candidate| candidate.label)
                .take(FIND_HITS)
                .collect(),
        )
        .with_recovery("`tool` `find` a few words of what you need, then `detail` a name it gives"))
    }

    fn candidates(&self) -> Vec<Candidate<'_>> {
        self.tools
            .iter()
            .map(|(name, filed)| Candidate {
                namespace: filed.namespace.as_str(),
                label: name.clone(),
                member: filed.namespace.member(name),
                about: &filed.def.description,
                text: searchable(filed),
            })
            .chain(self.skills.iter().map(|(name, admitted)| Candidate {
                namespace: ReservedNamespace::Skill.as_str(),
                label: format!("skill {name}"),
                member: name,
                about: &admitted.entry.disclosure,
                text: admitted.entry.disclosure.to_lowercase(),
            }))
            .collect()
    }
}

fn searchable(filed: &Filed) -> String {
    let parameters = filed
        .def
        .input_schema
        .as_map()
        .get("properties")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .map(|(key, property)| {
            let about = property
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or_default();
            format!("{key} {about}")
        })
        .collect::<Vec<_>>()
        .join(" ");
    format!("{} {parameters}", filed.def.description).to_lowercase()
}

fn score(words: &[String], candidate: &Candidate<'_>) -> u32 {
    let label = candidate.label.to_lowercase();
    let member = candidate.member.to_lowercase();
    words
        .iter()
        .filter_map(|word| grade(word, &label, &member, &candidate.text))
        .map(Grade::weight)
        .fold(0, u32::saturating_add)
}

fn grade(word: &str, label: &str, member: &str, text: &str) -> Option<Grade> {
    if word == label || word == member {
        Some(Grade::Exact)
    } else if label.starts_with(word) || member.starts_with(word) {
        Some(Grade::Prefix)
    } else if label.contains(word) {
        Some(Grade::Inside)
    } else if text.contains(word) {
        Some(Grade::Text)
    } else {
        None
    }
}

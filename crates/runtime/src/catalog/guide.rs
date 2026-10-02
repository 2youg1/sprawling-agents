// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What `describe` answers (runtime D22): an admitted capability's whole
//! guide when it is asked by its whole name, and otherwise the closest
//! names by a fixed keyword ranking over every admitted capability's
//! name and whole text, so a hint the dormant index cut is still found.

use kernel::{AxCode, AxError};

use super::{Catalog, Expansion, dormant};
use crate::tools::CallTool;

/// How many names one `describe` by keywords lists at most.
pub(crate) const DESCRIBE_HITS: usize = 8;

/// A keyword found in a capability's name counts this many times one
/// found only in its text: the name is what the model asked about.
const NAME_WEIGHT: u32 = 3;

impl Catalog {
    /// What `describe` answers for `asked`: the guide of the capability
    /// it names, or the closest names to its words.
    ///
    /// A tool's guide is its description and input schema as registered,
    /// unshortened, with how to run it; a skill's is its line and the
    /// name `read` opens it by, because `read` is the one door that
    /// opens a shelf; the mode and developer entries answer their own
    /// text.
    ///
    /// # Errors
    /// A tool schema that does not print as JSON, which its
    /// registration already printed once.
    pub fn describe(&self, asked: &str) -> Result<String, AxError> {
        let asked = asked.trim();
        if let Some(def) = self.tools.get(asked) {
            let schema = serde_json::to_string(&def.input_schema).map_err(|err| {
                AxError::failure(AxCode::InvalidArgs, "describe", asked.to_owned()).with_recovery(
                    format!(
                        "report this against runtime::catalog: a registered schema no \
                         longer prints as JSON ({err})"
                    ),
                )
            })?;
            let how = if self.is_core(asked) {
                "it is in your tool list; call it directly".to_owned()
            } else {
                format!(
                    "run it with `{}` and the arguments {{\"name\": \"{asked}\", \"args\": \
                     {{...}}}}",
                    CallTool::NAME
                )
            };
            return Ok(format!(
                "tool {asked}: {how}.\n{}\ninput schema: {schema}\n",
                def.description
            ));
        }
        // The index spells a skill `skill <name>`; either spelling names it.
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
        Ok(self.closest(asked))
    }

    /// The admitted capabilities ranked by how many of `asked`'s words
    /// they hold, highest first and then by name.
    fn closest(&self, asked: &str) -> String {
        let words: Vec<String> = asked
            .split(|c: char| !c.is_alphanumeric())
            .filter(|word| !word.is_empty())
            .map(str::to_lowercase)
            .collect();
        let mut ranked: Vec<(u32, String, &str)> = self
            .tools
            .iter()
            .map(|(name, def)| (name.clone(), def.description.as_str()))
            .chain(self.skills.iter().map(|(name, admitted)| {
                (format!("skill {name}"), admitted.entry.disclosure.as_str())
            }))
            .map(|(label, about)| (score(&words, &label, about), label, about))
            .filter(|(score, _, _)| *score > 0)
            .collect();
        ranked.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
        if ranked.is_empty() {
            return format!(
                "Nothing admitted here matches `{asked}`. The dormant index in your prompt \
                 names every capability this building admits beyond your tool list.\n"
            );
        }
        let mut out: String = ranked
            .iter()
            .take(DESCRIBE_HITS)
            .map(|(_, label, about)| format!("- {label}: {}\n", dormant::hint(about)))
            .collect();
        out.push_str("Describe one by its whole name for its guide.\n");
        out
    }
}

/// How well `label` and `about` answer `words`: each word found in the
/// label counts [`NAME_WEIGHT`], each found only in the text counts one.
fn score(words: &[String], label: &str, about: &str) -> u32 {
    let label = label.to_lowercase();
    let about = about.to_lowercase();
    words
        .iter()
        .map(|word| {
            if label.contains(word.as_str()) {
                NAME_WEIGHT
            } else {
                u32::from(about.contains(word.as_str()))
            }
        })
        .fold(0, u32::saturating_add)
}

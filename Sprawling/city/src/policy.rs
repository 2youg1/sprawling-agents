// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use kernel::layout::CityLayout;
use kernel::{
    Address, AxCode, AxError, BuildingPolicy, ContextLock, EgressAllowlist, EntropyAllowlist,
    WriteDomain,
};

mod cache;
mod desktop;
mod evaluate;
mod reach;
mod workspace;

pub use cache::RulesCache;
pub use desktop::{DESKTOP_SCOPE_FILE, desktop_scope_path, write_desktop_scope};

pub(crate) use evaluate::Written as RulesShape;
pub use evaluate::{
    RulesEdit, check_reading_room, evaluate, write_reading_room, write_rules_against,
    write_rules_against_with,
};
pub use reach::DomainReach;
pub use workspace::{
    ModelRecommendation, WorkspaceModelPolicy, WorkspaceRecommendations, inscribe_choice,
};

pub const RULES_FILE: &str = "RULES.toml";

const SUPERSEDED_FILE: &str = "BUILDING.md";

const HARNESS_MINUTES_DEFAULT: u32 = 60;

mod user_browser;

pub use user_browser::{UserBrowser, UserBrowserEndpoint};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelPool {
    Any,
    LocalOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildingRules {
    addr: Address,
    policy: BuildingPolicy,
    write_prefixes: Vec<Address>,
    reach: DomainReach,
    egress: Option<EgressAllowlist>,
    review: bool,

    browser: bool,

    usersbrowser: Option<UserBrowser>,

    desktop: bool,

    reading_room: Vec<String>,

    harness_minutes: Option<std::num::NonZeroU32>,

    workspace_models: Option<WorkspaceModelPolicy>,

    checkpoint_entropy_allow: EntropyAllowlist,

    context_lock: Option<ContextLock>,
}

impl BuildingRules {
    #[must_use]
    pub fn policy(&self) -> &BuildingPolicy {
        &self.policy
    }

    #[must_use]
    pub fn addr(&self) -> &Address {
        &self.addr
    }

    #[must_use]
    pub fn egress(&self) -> Option<&EgressAllowlist> {
        self.egress.as_ref()
    }

    #[must_use]
    pub fn review(&self) -> bool {
        self.review
    }

    #[must_use]
    pub fn browser(&self) -> bool {
        self.browser
    }

    #[must_use]
    pub fn usersbrowser(&self) -> Option<&UserBrowser> {
        self.usersbrowser.as_ref()
    }

    #[must_use]
    pub fn desktop(&self) -> bool {
        self.desktop
    }

    #[must_use]
    pub fn reading_room(&self) -> &[String] {
        &self.reading_room
    }

    #[must_use]
    pub fn harness_minutes(&self) -> u32 {
        self.harness_minutes
            .map_or(HARNESS_MINUTES_DEFAULT, std::num::NonZeroU32::get)
    }

    #[must_use]
    pub fn workspace_models(&self) -> Option<&WorkspaceModelPolicy> {
        self.workspace_models.as_ref()
    }

    #[must_use]
    pub fn checkpoint_entropy_allow(&self) -> &EntropyAllowlist {
        &self.checkpoint_entropy_allow
    }

    #[must_use]
    pub fn context_lock(&self) -> Option<ContextLock> {
        self.context_lock
    }

    #[must_use]
    pub fn reach(&self) -> DomainReach {
        self.reach
    }

    #[must_use]
    pub fn model_pool(&self) -> ModelPool {
        if self.policy.confidential {
            ModelPool::LocalOnly
        } else {
            ModelPool::Any
        }
    }

    pub fn write_domain(&self) -> Result<WriteDomain, AxError> {
        let mut prefixes = Vec::new();
        for prefix in &self.write_prefixes {
            if self.policy.confidential && !prefix.is_within(&self.addr) {
                return Err(AxError::failure(
                    AxCode::GateDenied,
                    "build the write domain of a confidential building",
                    format!("{} reaches outside {}", prefix.as_str(), self.addr.as_str()),
                )
                .with_recovery(
                    "remove that prefix, or drop `confidential = true` and say why in the file",
                ));
            }
            prefixes.push(prefix.clone());
        }
        if prefixes.is_empty() {
            prefixes.push(self.addr.clone());
        }
        match self.reach {
            DomainReach::Everything => WriteDomain::new(prefixes),
            DomainReach::Documents => WriteDomain::documents(prefixes),
        }
    }
}

#[must_use]
pub fn rules_path(city_root: &Path, addr: &Address) -> PathBuf {
    governed(city_root, addr).join(RULES_FILE)
}

#[must_use]
pub fn agents_path(city_root: &Path, addr: &Address) -> PathBuf {
    CityLayout::new(city_root)
        .scope(addr)
        .join(crate::spine_files::AGENTS_FILE)
}

pub fn room_agents_paths(
    city_root: &Path,
    building: &Address,
    room: &Address,
) -> Result<Vec<(Address, PathBuf)>, AxError> {
    let Some(below) = room
        .as_str()
        .strip_prefix(building.as_str())
        .and_then(|rest| rest.strip_prefix('/'))
    else {
        return Ok(Vec::new());
    };
    let mut level = building.as_str().to_owned();
    below
        .split('/')
        .map(|part| {
            level.push('/');
            level.push_str(part);
            let at = Address::parse(&level)?;
            let path = agents_path(city_root, &at);
            Ok((at, path))
        })
        .collect()
}

#[must_use]
pub fn city_agents_path(city_root: &Path) -> PathBuf {
    city_root.join(crate::spine_files::AGENTS_FILE)
}

fn superseded(city_root: &Path, addr: &Address) -> Option<PathBuf> {
    let layout = CityLayout::new(city_root);
    [
        governed(city_root, addr).join(SUPERSEDED_FILE),
        layout.scope(addr).join(SUPERSEDED_FILE),
    ]
    .into_iter()
    .find(|path| path.is_file())
}

pub(super) fn governed(city_root: &Path, addr: &Address) -> PathBuf {
    CityLayout::new(city_root)
        .scope(addr)
        .join(kernel::RESERVED_PREFIX)
}

pub fn load(city_root: &Path, addr: &Address) -> Result<BuildingRules, AxError> {
    let path = rules_path(city_root, addr);
    match std::fs::read_to_string(&path) {
        Ok(text) => evaluate(addr, &text),

        Err(err)
            if matches!(
                err.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
            ) =>
        {
            match superseded(city_root, addr) {
                Some(stale) => Err(AxError::failure(
                    AxCode::ConfigInvalid,
                    "read a building's rules",
                    addr.as_str().to_owned(),
                )
                .with_recovery(format!(
                    "move the settings in {} into {}, leaving the prose where it is; this \
                     version reads a building's rules as TOML in its reserved subtree, and \
                     rules it cannot find would load a confidential building as an ordinary \
                     one",
                    stale.display(),
                    path.display(),
                ))),
                None => Ok(BuildingRules {
                    addr: addr.clone(),
                    policy: BuildingPolicy::default(),
                    write_prefixes: Vec::new(),
                    reach: DomainReach::Everything,
                    egress: None,
                    review: false,
                    browser: false,
                    usersbrowser: None,
                    desktop: false,
                    reading_room: Vec::new(),
                    harness_minutes: None,
                    workspace_models: None,
                    checkpoint_entropy_allow: EntropyAllowlist::default(),
                    context_lock: None,
                }),
            }
        }
        Err(err) => Err(AxError::failure(
            AxCode::StorageFatal,
            "read a building's rules",
            format!("{}: {err}", path.display()),
        )
        .with_recovery("fix the file's permissions; a building's rules are not optional")),
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;

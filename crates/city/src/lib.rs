// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Space and identity: buildings, residents, spine files, archive,
//! library, building policy, schedule, wizard.

mod archive;
mod building;
mod check;
mod city_tool;
mod config_layers;
mod document;
mod gitignore;
mod governed;
mod handoff_form;
mod history;
mod identity;
mod library;
mod neighbourhood;
mod neighbours_tool;
mod policy;
mod resident;
mod room;
mod rules_tool;
mod schedule;
mod session;
mod spine_files;
mod vocation;
mod watch;
mod wizard;

pub use archive::Entry as ArchiveEntry;
pub use archive::Kind as ArchiveKind;
pub use archive::day_of;
pub use archive::entry as archive_entry;
pub use archive::file as file_archive;
pub use archive::index as archive_index;
pub use building::Written;
pub use building::adopt as adopt_building;
pub use building::adopted_payload as building_adopted_payload;
pub use building::all as buildings;
pub use building::configured_payload as building_configured_payload;
pub use building::created_payload as building_created_payload;
pub use building::remove as remove_building;
pub use building::removed_payload as building_removed_payload;
pub use building::{Building, BuildingTemplate, Removed, create as create_building};
pub use check::{Finding, Position, Report, check};
pub use city_tool::CityTool;
pub use config_layers::path as config_path;
pub use config_layers::write_second_threshold;
pub use config_layers::{CitySetting, write_city_setting};
pub use config_layers::{ConfigLayer, Layer, load as load_config};
pub use config_layers::{HostPermanence, RemoteRoute, remote_route};
pub use config_layers::{freeze_naming, keep_warm, own_layer, settled_harness, write_session};
pub use config_layers::{settled_effort, settled_second, write_mcp, write_sandbox};
pub use document::{Held, edit as edit_document, edit_against, revise as revise_document};
pub use governed::{Governed, PREFERENCES_FILE, write_governed};
pub use history::{History, has_history};
pub use identity::{DisplayName, NAME_MAX_CHARS, Naming, NamingEdit, NamingWritten};
pub use identity::{Unreadable, persona, read_naming, write_naming};
// Where each of these files sits is `kernel::layout`'s answer, and the
// names are re-exported rather than restated so that a caller reading
// `city::CONFIG_TOML` and the layout that places it cannot disagree.
pub use gitignore::place_city as ignore_city_records;
pub use gitignore::place_everywhere as keep_records_out_of_git;
pub use handoff_form::{HandoffSections, handoff_sections};
pub use kernel::layout::{ARCHIVE_DIR, BUILDING_SHELF, CONFIG_FILE, LIBRARY_DIR, URBANITE_FILE};
pub use library::{AuditState, Holding, Library, Shelf, audit_state};
pub use library::{Installed, Placed, PlannedInstall, SHIPPED_SECTION, Slot, shelve_shipped};
pub use library::{install as install_skill, plan_install as plan_skill_install};
pub use neighbourhood::{Neighbour, Neighbourhood, Occupancy};
pub use neighbours_tool::NeighboursTool;
pub use policy::{BuildingRules, DomainReach, ModelPool, RULES_FILE, RulesCache};
pub use policy::{DESKTOP_SCOPE_FILE, desktop_scope_path, write_desktop_scope};
pub use policy::{UserBrowser, UserBrowserEndpoint};
pub use policy::{agents_path, city_agents_path, evaluate, load, rules_path};
pub use policy::{write_rules, write_rules_against};
pub use resident::{Dossier, Identity, Resident, urbanite_path};
pub use room::all as rooms;
pub use room::claim as claim_room;
pub use room::open as open_room;
pub use rules_tool::RulesTool;
pub use schedule::{Cadence, Entry, SCHEDULE_FILE, Schedule, schedule_path};
pub use session::{clear_session, forget_shape};
pub use spine_files::CITY_TEMPLATE;
pub use spine_files::{AGENTS_FILE, CITY_FILE, CLERK_FILE, HANDOFF_FILE, MAYOR_FILE};
pub use spine_files::{JobBrief, ROADMAP_FILE, RunBrief};
pub use spine_files::{MEMO_FILE, SPEC_FILE};
pub use spine_files::{hall_identity_path, lay_out_hall_identities};
pub use spine_files::{handoff, handoff_path};
pub use spine_files::{job_path, norms, roadmap, roadmap_path, write_brief, write_job};
pub use vocation::{Vocation, vocation_of};
pub use watch::{Link, Source, WATCH_FILE, Watch, watch_path};
pub use wizard::{CityPlan, Standing, survey};

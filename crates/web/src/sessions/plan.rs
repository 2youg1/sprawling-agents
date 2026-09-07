// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The box that starts work: guess, choose, and what each field means.

use crate::app::Snapshot;
use crate::lang::Msg;

/// How many finished sessions the second table holds.
///
/// Eight rather than all of them: what ended is context for what is
/// happening, and a list that grows without bound turns the page a
/// person opens to act into a page they have to scroll.
pub const ENDED_ROWS: usize = 8;

/// The mode a piece of work runs in when nobody has said otherwise.
///
/// `runtime::Mode` is the authority for the set; this is the authority
/// for which one a person who says nothing gets. Build, because it is
/// the mode that produces something, and the one a person who has not
/// yet learned the others meant.
pub const DEFAULT_MODE: &str = "build";

/// One decision in the sentence under the box.
///
/// Three, and they are the three a Run cannot start without. Money is
/// not among them: this city has no budget lock, the receiver of a cost
/// figure is an agent rather than a brake, and a person typing here has
/// no way to know the number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    /// Which room the work goes to.
    Room,
    /// Which mode it runs in.
    Mode,
    /// How hard the model is asked to think.
    Effort,
}

impl Field {
    /// Every field, in the order the sentence reads them.
    pub const ALL: [Field; 3] = [Field::Room, Field::Mode, Field::Effort];

    /// The phrase this field's word sits inside.
    #[must_use]
    pub fn sentence(self) -> Msg {
        match self {
            Self::Room => Msg::ComposerSendTo,
            Self::Mode => Msg::ComposerAs,
            Self::Effort => Msg::ComposerThink,
        }
    }

    /// The slot inside that phrase which this field fills.
    #[must_use]
    pub fn slot(self) -> &'static str {
        match self {
            Self::Room => "room",
            Self::Mode => "mode",
            Self::Effort => "effort",
        }
    }

    /// What the field's own control is labelled when it opens.
    #[must_use]
    pub fn label(self) -> Msg {
        match self {
            Self::Room => Msg::ComposerRoomFor,
            Self::Mode => Msg::ComposerModeFor,
            Self::Effort => Msg::ComposerEffortFor,
        }
    }
}

/// What the composer will send, and which parts of it a person chose.
///
/// The distinction is the point: `chosen` is what makes a decision draw
/// differently from a guess, and it is a fact about how the value got
/// here rather than about the value.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Plan {
    pub room: String,
    pub mode: String,
    pub effort: String,
    /// The fields a person set for themselves.
    pub(super) chosen: Vec<Field>,
}

impl Plan {
    /// The plan this city would guess, from what it has already seen.
    ///
    /// Rules, and named as rules: the room a person last sent work to,
    /// the mode that produces something, and the depth this city was
    /// configured with. When a model does the inferring instead
    /// (`bin::assembly`), it answers the same three and the city refuses
    /// rather than guessing when it cannot.
    ///
    /// A city with no history guesses nothing for the room and says so
    /// by leaving it empty, which opens that one control. Inventing a
    /// building name would be the interface answering for the person.
    #[must_use]
    pub fn guessed(snapshot: &Snapshot, effort: &str) -> Self {
        let room = latest_room(snapshot).unwrap_or_default();
        Self {
            room,
            mode: DEFAULT_MODE.to_owned(),
            effort: effort.to_owned(),
            chosen: Vec::new(),
        }
    }

    /// The value in one field.
    #[must_use]
    pub fn value(&self, field: Field) -> &str {
        match field {
            Field::Room => &self.room,
            Field::Mode => &self.mode,
            Field::Effort => &self.effort,
        }
    }

    /// Records a person's own choice, which is what stops the word being
    /// drawn as a guess. Setting a field to what it already guessed is
    /// still a decision: they read it and agreed.
    pub fn choose(&mut self, field: Field, value: String) {
        match field {
            Field::Room => self.room = value,
            Field::Mode => self.mode = value,
            Field::Effort => self.effort = value,
        }
        if !self.chosen.contains(&field) {
            self.chosen.push(field);
        }
    }

    /// Whether this word came from the person rather than from the city.
    #[must_use]
    pub fn is_chosen(&self, field: Field) -> bool {
        self.chosen.contains(&field)
    }

    /// The class the word is drawn with. One producer, so the two
    /// underlines cannot come apart from the two meanings.
    #[must_use]
    pub fn ink(&self, field: Field) -> &'static str {
        if self.is_chosen(field) {
            "chosen"
        } else {
            "guess"
        }
    }
}

/// Which rung of the readiness ladder this city is on.
///
/// **Not wizard steps.** Each one is a fact that is true right now, so a
/// person who attaches a provider in another window finds the first rung
/// gone when they come back. A wizard would have stored its own progress
/// — a second authority on how far along this city is — and the stored
/// copy is what goes stale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rung {
    /// No model answers for the tag a run thinks with.
    NoModel,
    /// There is a model and nowhere to put work.
    NoBuilding,
    /// It can send work out and never has.
    NeverSent,
    /// It has sent work out. The ladder is behind this city.
    Working,
}

/// Where this city stands, from what it holds right now.
///
/// Ordered by what blocks what: a city with no model cannot be helped by
/// being told it has no buildings, and a person given two problems at
/// once solves neither. One rung, one action.
#[must_use]
pub fn rung(
    endpoints: Option<&channels::EndpointsAnswer>,
    city: Option<&channels::CityAnswer>,
    snapshot: &Snapshot,
) -> Rung {
    // `None` is "not asked yet", which is not the same as "none
    // attached". A page that showed the first rung while the answer was
    // in flight would tell a working city it cannot work.
    let answered = match (endpoints, city) {
        (Some(endpoints), Some(city)) => (endpoints, city),
        _ => return Rung::Working,
    };
    let (endpoints, city) = answered;
    if !crate::settings::can_dispatch(endpoints) {
        return Rung::NoModel;
    }
    if city.buildings.is_empty() {
        return Rung::NoBuilding;
    }
    if snapshot.runs().next().is_none() {
        return Rung::NeverSent;
    }
    Rung::Working
}

/// The room work was most recently sent to.
///
/// The strongest signal this city has about where the next piece goes,
/// and it costs nothing: a person working on one thing sends several
/// pieces of work to the same place.
#[must_use]
pub fn latest_room(snapshot: &Snapshot) -> Option<String> {
    snapshot
        .runs()
        .filter_map(|(_, row)| row.addr.as_ref().map(|addr| (row.started_at_seq, addr)))
        .max_by_key(|(seq, _)| *seq)
        .map(|(_, addr)| addr.as_str().to_owned())
}

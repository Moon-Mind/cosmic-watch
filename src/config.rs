// SPDX-License-Identifier: MPL-2.0

use cosmic::cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, CosmicConfigEntry};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, CosmicConfigEntry, Eq, PartialEq, Default)]
#[version = 1]
pub struct Config {
    pub alarms: Vec<AlarmConfig>,
    pub world_clocks: Vec<WorldClockConfig>,
    pub timer_presets: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct AlarmConfig {
    pub id: u32,
    pub hour: u32,
    pub minute: u32,
    pub label: String,
    pub enabled: bool,
    pub repeat_days: [bool; 7],
    pub snooze_minutes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct WorldClockConfig {
    pub name: String,
    pub timezone: String,
}

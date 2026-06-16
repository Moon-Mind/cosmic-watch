// SPDX-License-Identifier: MPL-2.0

use crate::config;
use crate::config::Config;
use crate::fl;
use crate::notifications;
use chrono::Timelike;
use chrono::TimeZone;
use chrono::Offset;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::keyboard;
use cosmic::iced::{Alignment, Length, Subscription};
use cosmic::prelude::*;
use cosmic::widget::{self, canvas, icon, menu, nav_bar};
use cosmic::{cosmic_theme, theme};
use std::borrow::Cow;
use std::time::Duration;

const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const APP_ICON: &[u8] = include_bytes!("../resources/icons/hicolor/scalable/apps/icon.svg");

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime.
    core: cosmic::Core,
    /// Display a context drawer with the designated page if defined.
    context_page: ContextPage,
    /// Contains items assigned to the nav bar panel.
    nav: nav_bar::Model,

    /// Configuration data that persists between application runs.
    config: Config,
    /// Current time for display
    current_time: chrono::DateTime<chrono::Local>,
    /// Stopwatch state
    stopwatch_time: Duration,
    stopwatch_str: String,
    stopwatch_running: bool,
    stopwatch_paused: bool,
    stopwatch_laps: Vec<LapEntry>,
    /// Timer states (multiple timers supported)
    timers: Vec<TimerItem>,
    /// Alarm state
    alarms: Vec<AlarmItem>,
    next_alarm_id: u32,
    /// Alarm editing state
    editing_alarm: Option<AlarmEdit>,
    /// World clock timezones
    world_clocks: Vec<WorldClockItem>,
    /// Current page for subscription control
    current_page: Page,
    /// City search text
    city_search: String,
    /// Show city picker overlay
    show_city_picker: bool,
    /// Hovered clock index
    hovered_clock: Option<usize>,

}

#[derive(Clone, Debug)]
pub struct AlarmItem {
    pub id: u32,
    pub time: chrono::NaiveTime,
    pub label: String,
    pub enabled: bool,
    pub repeat_days: [bool; 7],
    pub snooze_minutes: u32,
    pub sound: String,
}

#[derive(Clone, Debug)]
pub struct AlarmEdit {
    pub id: Option<u32>, // None for new alarm
    pub hour: u32,       // 0-23
    pub minute: u32,     // 0-59
    pub label: String,
    pub repeat_days: [bool; 7], // Sun-Sat
    pub sound: String,
    pub snooze_enabled: bool,
    pub snooze_minutes: u32,
}

const ALARM_SOUNDS: &[&str] = &[
    "Radar", "Typewriter", "Storytime", "Silk", "Moment",
    "Presto", "Syncopation", "Stargaze", "Harpsichord", "Pluck",
];

const SNOOZE_OPTIONS: &[u32] = &[1, 3, 5, 9, 15, 30, 60];
const SNOOZE_STR: &[&str] = &["1 min", "3 min", "5 min", "9 min", "15 min", "30 min", "60 min"];

const DAY_LABELS: &[&str] = &["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];



#[derive(Clone, Debug)]
pub struct WorldClockItem {
    pub name: String,
    pub country: String,
    pub timezone: String,
    pub offset_hours: i32,
    pub parsed_tz: Option<chrono_tz::Tz>,
}

#[derive(Clone, Debug, Default)]
pub struct LapEntry {
    pub split: Duration,
    pub total: Duration,
}

#[derive(Clone, Debug)]
pub struct TimerItem {
    pub duration: Duration,
    pub remaining: Duration,
    pub running: bool,
    pub paused: bool,
    pub name: String,
    pub edit_hours: u32,
    pub edit_minutes: u32,
    pub edit_seconds: u32,
    pub active_segment: usize, // 0=hours, 1=minutes, 2=seconds
    pub display_str: String,
}

impl Default for TimerItem {
    fn default() -> Self {
        TimerItem {
            duration: Duration::from_secs(300),
            remaining: Duration::from_secs(300),
            running: false,
            paused: false,
            name: String::new(),
            edit_hours: 0,
            edit_minutes: 5,
            edit_seconds: 0,
            active_segment: 1,
            display_str: String::from("00:05:00"),
        }
    }
}

fn format_duration(d: Duration) -> String {
    let total = d.as_secs();
    format!("{:02}:{:02}:{:02}", total / 3600, (total % 3600) / 60, total % 60)
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    OpenRepositoryUrl,
    ToggleContextPage(ContextPage),
    UpdateConfig(Config),
    SaveConfig,
    LaunchUrl(String),
    UpdateTime,
    // Stopwatch messages
    StartStopwatch,
    StopStopwatch,
    ResetStopwatch,
    RecordLap,
    // Timer messages (index-based for multiple timers)
    AddTimer,
    DeleteTimer(usize),
    TimerStart(usize),
    TimerStop(usize),
    TimerPause(usize),
    TimerReset(usize),
    TimerSetSegment(usize, usize, u32),
    TimerSetName(usize, String),
    TimerSelectSegment(usize, usize),
    // Alarm messages
    AddAlarm,
    EditAlarm(u32),
    DeleteAlarm(u32),
    ToggleAlarm(u32),
    SaveAlarm,
    CancelAlarmEdit,
    AlarmEditLabel(String),
    AlarmEditRepeatDay(u8, bool),
    AlarmEditEveryDay(bool),
    AlarmEditSnoozeMinutes(u32),
    AlarmEditSound(String),
    AlarmEditSnoozeEnabled(bool),
    AlarmEditHour(u32),
    AlarmEditMinute(u32),
    SnoozeAlarm(u32),
    // World clock messages
    AddWorldClock,
    DeleteWorldClock(usize),
    SelectTimezone(usize, String),
    SearchCity(String),
    SelectSearchedCity(usize),
    ShowCityPicker,
    SortWorldClocks,
    MoveWorldClockUp(usize),
    MoveWorldClockDown(usize),
    HoverClock(Option<usize>),
    // Navigation
    NavTo(usize),
}

const CITIES: &[(&str, &str, &str)] = &[
    ("New York", "United States", "America/New_York"),
    ("London", "United Kingdom", "Europe/London"),
    ("Tokyo", "Japan", "Asia/Tokyo"),
    ("Paris", "France", "Europe/Paris"),
    ("Sydney", "Australia", "Australia/Sydney"),
    ("Dubai", "United Arab Emirates", "Asia/Dubai"),
    ("Moscow", "Russia", "Europe/Moscow"),
    ("Shanghai", "China", "Asia/Shanghai"),
    ("Los Angeles", "United States", "America/Los_Angeles"),
    ("Berlin", "Germany", "Europe/Berlin"),
    ("Mumbai", "India", "Asia/Kolkata"),
    ("Hong Kong", "China", "Asia/Hong_Kong"),
    ("Singapore", "Singapore", "Asia/Singapore"),
    ("São Paulo", "Brazil", "America/Sao_Paulo"),
    ("Toronto", "Canada", "America/Toronto"),
    ("Seoul", "South Korea", "Asia/Seoul"),
    ("Istanbul", "Turkey", "Europe/Istanbul"),
    ("Mexico City", "Mexico", "America/Mexico_City"),
    ("Rome", "Italy", "Europe/Rome"),
    ("Madrid", "Spain", "Europe/Madrid"),
    ("Amsterdam", "Netherlands", "Europe/Amsterdam"),
    ("Bangkok", "Thailand", "Asia/Bangkok"),
    ("Buenos Aires", "Argentina", "America/Argentina/Buenos_Aires"),
    ("Chicago", "United States", "America/Chicago"),
    ("Jakarta", "Indonesia", "Asia/Jakarta"),
    ("Kuala Lumpur", "Malaysia", "Asia/Kuala_Lumpur"),
    ("Lagos", "Nigeria", "Africa/Lagos"),
    ("Cairo", "Egypt", "Africa/Cairo"),
    ("Delhi", "India", "Asia/Kolkata"),
    ("Vancouver", "Canada", "America/Vancouver"),
    ("San Francisco", "United States", "America/Los_Angeles"),
    ("Miami", "United States", "America/New_York"),
    ("Denver", "United States", "America/Denver"),
    ("Phoenix", "United States", "America/Phoenix"),
    ("Houston", "United States", "America/Chicago"),
    ("Boston", "United States", "America/New_York"),
    ("Seattle", "United States", "America/Los_Angeles"),
    ("Montreal", "Canada", "America/Montreal"),
    ("Vienna", "Austria", "Europe/Vienna"),
    ("Prague", "Czech Republic", "Europe/Prague"),
    ("Warsaw", "Poland", "Europe/Warsaw"),
    ("Budapest", "Hungary", "Europe/Budapest"),
    ("Stockholm", "Sweden", "Europe/Stockholm"),
    ("Oslo", "Norway", "Europe/Oslo"),
    ("Copenhagen", "Denmark", "Europe/Copenhagen"),
    ("Helsinki", "Finland", "Europe/Helsinki"),
    ("Athens", "Greece", "Europe/Athens"),
    ("Lisbon", "Portugal", "Europe/Lisbon"),
    ("Dublin", "Ireland", "Europe/Dublin"),
    ("Zurich", "Switzerland", "Europe/Zurich"),
    ("Brussels", "Belgium", "Europe/Brussels"),
    ("Luxembourg", "Luxembourg", "Europe/Luxembourg"),
    ("Monaco", "Monaco", "Europe/Monaco"),
    ("Reykjavik", "Iceland", "Atlantic/Reykjavik"),
    ("Cape Town", "South Africa", "Africa/Johannesburg"),
    ("Nairobi", "Kenya", "Africa/Nairobi"),
    ("Casablanca", "Morocco", "Africa/Casablanca"),
    ("Tel Aviv", "Israel", "Asia/Jerusalem"),
    ("Amman", "Jordan", "Asia/Amman"),
    ("Riyadh", "Saudi Arabia", "Asia/Riyadh"),
    ("Tehran", "Iran", "Asia/Tehran"),
    ("Karachi", "Pakistan", "Asia/Karachi"),
    ("Dhaka", "Bangladesh", "Asia/Dhaka"),
    ("Yangon", "Myanmar", "Asia/Yangon"),
    ("Hanoi", "Vietnam", "Asia/Ho_Chi_Minh"),
    ("Manila", "Philippines", "Asia/Manila"),
    ("Taipei", "Taiwan", "Asia/Taipei"),
    ("Osaka", "Japan", "Asia/Tokyo"),
    ("Auckland", "New Zealand", "Pacific/Auckland"),
    ("Fiji", "Fiji", "Pacific/Fiji"),
    ("Honolulu", "United States", "Pacific/Honolulu"),
    ("Anchorage", "United States", "America/Anchorage"),
    ("Perth", "Australia", "Australia/Perth"),
    ("Melbourne", "Australia", "Australia/Melbourne"),
    ("Brisbane", "Australia", "Australia/Brisbane"),
    ("Calcutta", "India", "Asia/Kolkata"),
    ("Colombo", "Sri Lanka", "Asia/Colombo"),
    ("Kathmandu", "Nepal", "Asia/Kathmandu"),
];

struct AnalogClock {
    time: chrono::NaiveTime,
    is_day: bool,
}

impl<Message> canvas::Program<Message, cosmic::Theme, cosmic::Renderer> for AnalogClock {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &cosmic::Renderer,
        _theme: &cosmic::Theme,
        bounds: cosmic::iced::Rectangle,
        _cursor: cosmic::iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry<cosmic::Renderer>> {
        use std::f32::consts::PI;

        let mut frame = canvas::Frame::new(renderer, bounds.size());

        let center = cosmic::iced::Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let radius = bounds.width.min(bounds.height) / 2.0 - 10.0;

        // Background - pure white for day, pure black for night
        let bg_color = if self.is_day {
            cosmic::iced::Color::WHITE
        } else {
            cosmic::iced::Color::BLACK
        };
        frame.fill(&canvas::Path::circle(center, radius), bg_color);

        // Outer ring
        let ring_color = if self.is_day {
            cosmic::iced::Color::from_rgba(0.0, 0.0, 0.0, 0.12)
        } else {
            cosmic::iced::Color::from_rgba(1.0, 1.0, 1.0, 0.2)
        };
        frame.stroke(
            &canvas::Path::circle(center, radius - 2.0),
            canvas::Stroke::default().with_color(ring_color).with_width(1.0),
        );

        // Hour ticks (12 ticks, thicker at 3/6/9/12)
        for i in 0..12 {
            let radians = (i as f32 * 30.0 - 90.0) * PI / 180.0;
            let (inner, outer, width) = if i % 3 == 0 {
                (radius * 0.82, radius * 0.95, 3.0)
            } else {
                (radius * 0.88, radius * 0.95, 1.5)
            };
            let tick_color = if self.is_day {
                cosmic::iced::Color::from_rgb8(0x33, 0x33, 0x33)
            } else {
                cosmic::iced::Color::from_rgb8(0xCC, 0xCC, 0xCC)
            };
            let p1 = cosmic::iced::Point::new(center.x + inner * radians.cos(), center.y + inner * radians.sin());
            let p2 = cosmic::iced::Point::new(center.x + outer * radians.cos(), center.y + outer * radians.sin());
            frame.stroke(
                &canvas::Path::line(p1, p2),
                canvas::Stroke::default().with_color(tick_color).with_width(width).with_line_cap(canvas::LineCap::Round),
            );
        }

        // Hour hand
        let hour_angle = ((self.time.hour() as f32 % 12.0) * 30.0 + self.time.minute() as f32 * 0.5 - 90.0) * PI / 180.0;
        let hour_len = radius * 0.5;
        let hand_color = if self.is_day {
            cosmic::iced::Color::from_rgb8(0x11, 0x11, 0x11)
        } else {
            cosmic::iced::Color::from_rgb8(0xEE, 0xEE, 0xEE)
        };
        let p1 = cosmic::iced::Point::new(
            center.x - hour_len * 0.3 * hour_angle.cos(),
            center.y - hour_len * 0.3 * hour_angle.sin(),
        );
        let p2 = cosmic::iced::Point::new(
            center.x + hour_len * hour_angle.cos(),
            center.y + hour_len * hour_angle.sin(),
        );
        frame.stroke(
            &canvas::Path::line(p1, p2),
            canvas::Stroke::default().with_color(hand_color).with_width(4.0).with_line_cap(canvas::LineCap::Round),
        );

        // Minute hand
        let min_angle = (self.time.minute() as f32 * 6.0 + self.time.second() as f32 * 0.1 - 90.0) * PI / 180.0;
        let min_len = radius * 0.7;
        let mp1 = cosmic::iced::Point::new(
            center.x - min_len * 0.1 * min_angle.cos(),
            center.y - min_len * 0.1 * min_angle.sin(),
        );
        let mp2 = cosmic::iced::Point::new(
            center.x + min_len * min_angle.cos(),
            center.y + min_len * min_angle.sin(),
        );
        frame.stroke(
            &canvas::Path::line(mp1, mp2),
            canvas::Stroke::default().with_color(hand_color).with_width(2.5).with_line_cap(canvas::LineCap::Round),
        );

        // Second hand (red, sweeping)
        let sec_angle = (self.time.second() as f32 * 6.0 - 90.0) * PI / 180.0;
        let sec_len = radius * 0.8;
        let red = cosmic::iced::Color::from_rgb8(0xE4, 0x35, 0x35);
        let p1 = cosmic::iced::Point::new(
            center.x - sec_len * 0.2 * sec_angle.cos(),
            center.y - sec_len * 0.2 * sec_angle.sin(),
        );
        let p2 = cosmic::iced::Point::new(
            center.x + sec_len * sec_angle.cos(),
            center.y + sec_len * sec_angle.sin(),
        );
        frame.stroke(
            &canvas::Path::line(p1, p2),
            canvas::Stroke::default().with_color(red).with_width(1.5).with_line_cap(canvas::LineCap::Round),
        );

        // Center dot
        frame.fill(&canvas::Path::circle(center, 3.0), red);

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        _bounds: cosmic::iced::Rectangle,
        _cursor: cosmic::iced::mouse::Cursor,
    ) -> cosmic::iced::mouse::Interaction {
        cosmic::iced::mouse::Interaction::default()
    }
}

/// Create a COSMIC application from the app model
impl cosmic::Application for AppModel {
    /// The async executor that will be used to run your application's commands.
    type Executor = cosmic::executor::Default;

    /// Data that your application receives to its init method.
    type Flags = ();

    /// Messages which the application and its widgets will emit.
    type Message = Message;

    /// Unique identifier in RDNN (reverse domain name notation) format.
    const APP_ID: &'static str = "com.github.Moon-Mind.cosmic-watch";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    /// Initializes the application with any given flags and startup commands.
    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        // Create a nav bar with all pages
        let mut nav = nav_bar::Model::default();

        nav.insert()
            .text(fl!("world-clock"))
            .data::<Page>(Page::WorldClock)
            .icon(icon::from_name("preferences-system-time-symbolic"))
            .activate();

        nav.insert()
            .text(fl!("alarms"))
            .data::<Page>(Page::Alarm)
            .icon(icon::from_name("alarm-symbolic"));

        nav.insert()
            .text(fl!("stopwatch"))
            .data::<Page>(Page::Stopwatch)
            .icon(icon::from_name("accessories-clock-symbolic"));

        nav.insert()
            .text(fl!("timer"))
            .data::<Page>(Page::Timer)
            .icon(icon::from_name("appointment-soon-symbolic"));

        // Construct the app model with the runtime's core.
        let config: Config = cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
            .map(|context| match Config::get_entry(&context) {
                Ok(config) => config,
                Err((_errors, config)) => config,
            })
            .unwrap_or_default();

        let alarms: Vec<AlarmItem> = config.alarms.iter().map(|ac| AlarmItem {
            id: ac.id,
            time: chrono::NaiveTime::from_hms_opt(ac.hour, ac.minute, 0).unwrap_or_default(),
            label: ac.label.clone(),
            enabled: ac.enabled,
            repeat_days: ac.repeat_days,
            snooze_minutes: ac.snooze_minutes,
            sound: if ac.sound.is_empty() { String::from("Radar") } else { ac.sound.clone() },
        }).collect();

        let next_alarm_id = alarms.iter().map(|a| a.id).max().unwrap_or(0) + 1;

        let mut world_clocks = vec![
            WorldClockItem {
                name: String::from("Local"),
                country: String::new(),
                timezone: String::from("Local"),
                offset_hours: 0,
                parsed_tz: None,
            },
        ];
        for wc in &config.world_clocks {
            if let Ok(tz) = wc.timezone.parse::<chrono_tz::Tz>() {
                let offset = tz.offset_from_utc_datetime(&chrono::Utc::now().naive_utc()).fix().local_minus_utc();
                world_clocks.push(WorldClockItem {
                    name: wc.name.clone(),
                    country: wc.country.clone(),
                    timezone: wc.timezone.clone(),
                    offset_hours: offset / 3600,
                    parsed_tz: Some(tz),
                });
            }
        }

        let current_page = Page::WorldClock;

        let mut app = AppModel {
            core,
            context_page: ContextPage::default(),
            nav,
            config,
            current_time: chrono::Local::now(),
            stopwatch_time: Duration::default(),
            stopwatch_str: String::from("00:00.00"),
            stopwatch_running: false,
            stopwatch_paused: false,
            stopwatch_laps: Vec::new(),
            timers: vec![TimerItem::default()],
            alarms,
            next_alarm_id,
            editing_alarm: None,
            world_clocks,
            current_page,
            city_search: String::new(),
            show_city_picker: false,
            hovered_clock: None,

        };

        let command = app.update_title();

        (app, command)
    }

    /// Elements to pack at the start of the header bar.
    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        let menu_bar = menu::bar(vec![menu::Tree::with_children(
            menu::root(fl!("view")).apply(Element::from),
            menu::items(
                &std::collections::HashMap::new(),
                vec![menu::Item::Button(fl!("about"), None, MenuAction::About)],
            ),
        )]);

        vec![menu_bar.into()]
    }

    /// Enables the COSMIC application to create a nav bar with this model.
    fn nav_model(&self) -> Option<&nav_bar::Model> {
        Some(&self.nav)
    }

    fn context_drawer(&self) -> Option<cosmic::app::ContextDrawer<'_, Self::Message>> {
        if !self.core.window.show_context {
            return None;
        }

        Some(match self.context_page {
            ContextPage::About => cosmic::app::context_drawer::context_drawer(
                self.about(),
                Message::ToggleContextPage(ContextPage::About)
            ).title(fl!("about")),
        })
    }

    /// Describes the interface based on the current state of the application model.
    fn view(&self) -> Element<'_, Self::Message> {
        let page = self
            .nav
            .data::<Page>(self.nav.active())
            .cloned()
            .unwrap_or_default();

        match page {
            Page::WorldClock => self.world_clock_view(),
            Page::Alarm => self.alarm_view(),
            Page::Stopwatch => self.stopwatch_view(),
            Page::Timer => self.timer_view(),
        }
    }

    /// Register subscriptions for this application.
    fn subscription(&self) -> Subscription<Self::Message> {
        let mut subscriptions = vec![
            cosmic::iced::time::every(Duration::from_secs(1)).map(|_| Message::UpdateTime),
            self.core()
                .watch_config::<Config>(Self::APP_ID)
                .map(|update| Message::UpdateConfig(update.config)),
            cosmic::iced::event::listen().map(|event| {
                if let cosmic::iced::Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event {
                    if modifiers.control() {
                        match key {
                            cosmic::iced::keyboard::Key::Character(c) => {
                                match c.as_str() {
                                    "1" => return Message::NavTo(0),
                                    "2" => return Message::NavTo(1),
                                    "3" => return Message::NavTo(2),
                                    "4" => return Message::NavTo(3),
                                    _ => {}
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Message::UpdateTime
            }),
        ];

        // Add more frequent updates for stopwatch and timer
        let needs_100ms = (self.stopwatch_running && !self.stopwatch_paused)
            || self.timers.iter().any(|t| t.running && !t.paused);
        if needs_100ms {
            subscriptions.push(
                cosmic::iced::time::every(Duration::from_millis(100)).map(|_| Message::UpdateTime),
            );
        }

        Subscription::batch(subscriptions)
    }

    /// Handles messages emitted by the application and its widgets.
    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::OpenRepositoryUrl => {
                _ = open::that_detached(REPOSITORY);
            }

            Message::ToggleContextPage(context_page) => {
                if self.context_page == context_page {
                    self.core.window.show_context = !self.core.window.show_context;
                } else {
                    self.context_page = context_page;
                    self.core.window.show_context = true;
                }
            }

            Message::UpdateConfig(config) => {
                self.config = config;
            }

            Message::SaveConfig => {
                self.save_config();
            }

            Message::LaunchUrl(url) => match open::that_detached(&url) {
                Ok(()) => {}
                Err(err) => {
                    eprintln!("failed to open {url:?}: {err}");
                }
            },

            Message::UpdateTime => {
                self.current_time = chrono::Local::now();
                
                if self.stopwatch_running && !self.stopwatch_paused {
                    self.stopwatch_time += Duration::from_millis(100);
                    let t = self.stopwatch_time;
                    self.stopwatch_str = format!("{:02}:{:02}.{:02}",
                        t.as_secs() / 60, t.as_secs() % 60, (t.as_millis() % 1000) / 10);
                }
                
                // Update all running timers
                for timer in &mut self.timers {
                    if timer.running && !timer.paused && timer.remaining > Duration::default() {
                        timer.remaining = timer.remaining.saturating_sub(Duration::from_millis(100));
                        timer.display_str = format_duration(timer.remaining);
                        if timer.remaining == Duration::default() {
                            timer.running = false;
                            notifications::send_timer_notification();
                        }
                    }
                }

                // Check for alarm triggers
                self.check_alarms();
            }

            Message::StartStopwatch => {
                self.stopwatch_running = true;
                self.stopwatch_paused = false;
            }

            Message::StopStopwatch => {
                self.stopwatch_running = false;
                self.stopwatch_paused = false;
                notifications::send_stopwatch_notification(&self.stopwatch_str);
            }

            Message::ResetStopwatch => {
                self.stopwatch_running = false;
                self.stopwatch_paused = false;
                self.stopwatch_time = Duration::default();
                self.stopwatch_laps.clear();
            }

            Message::RecordLap => {
                if self.stopwatch_running && !self.stopwatch_paused {
                    let total = self.stopwatch_time;
                    let split = if let Some(prev) = self.stopwatch_laps.last() {
                        total.saturating_sub(prev.total)
                    } else {
                        total
                    };
                    self.stopwatch_laps.push(LapEntry { split, total });
                }
            }

            Message::AddTimer => {
                self.timers.push(TimerItem::default());
            }

            Message::DeleteTimer(index) => {
                if index < self.timers.len() {
                    self.timers.remove(index);
                }
            }

            Message::TimerStart(index) => {
                if let Some(timer) = self.timers.get_mut(index) {
                    if timer.paused {
                        timer.paused = false;
                        timer.running = true;
                    } else if !timer.running {
                        // Commit edit values as the duration
                        timer.duration = Duration::from_secs(
                            timer.edit_hours as u64 * 3600
                            + timer.edit_minutes as u64 * 60
                            + timer.edit_seconds as u64
                        );
                        if timer.duration == Duration::default() {
                            timer.duration = Duration::from_secs(60);
                        }
                        timer.remaining = timer.duration;
                        timer.running = true;
                    }
                }
            }

            Message::TimerStop(index) => {
                if let Some(timer) = self.timers.get_mut(index) {
                    timer.running = false;
                    timer.paused = false;
                }
            }

            Message::TimerPause(index) => {
                if let Some(timer) = self.timers.get_mut(index) {
                    if timer.running {
                        timer.paused = !timer.paused;
                    }
                }
            }

            Message::TimerReset(index) => {
                if let Some(timer) = self.timers.get_mut(index) {
                    timer.running = false;
                    timer.paused = false;
                    timer.remaining = timer.duration;
                }
            }

            Message::AddAlarm => {
                self.editing_alarm = Some(AlarmEdit {
                    id: None,
                    hour: self.current_time.hour(),
                    minute: self.current_time.minute(),
                    label: String::new(),
                    repeat_days: [false; 7],
                    sound: String::from("Radar"),
                    snooze_enabled: true,
                    snooze_minutes: 9,
                });
            }

            Message::EditAlarm(id) => {
                if let Some(alarm) = self.alarms.iter().find(|a| a.id == id) {
                    self.editing_alarm = Some(AlarmEdit {
                        id: Some(id),
                        hour: alarm.time.hour(),
                        minute: alarm.time.minute(),
                        label: alarm.label.clone(),
                        repeat_days: alarm.repeat_days,
                        sound: if alarm.sound.is_empty() { String::from("Radar") } else { alarm.sound.clone() },
                        snooze_enabled: alarm.snooze_minutes > 0,
                        snooze_minutes: if alarm.snooze_minutes > 0 { alarm.snooze_minutes } else { 9 },
                    });
                }
            }

            Message::DeleteAlarm(id) => {
                self.alarms.retain(|alarm| alarm.id != id);
                self.save_config();
            }

            Message::ToggleAlarm(id) => {
                if let Some(alarm) = self.alarms.iter_mut().find(|a| a.id == id) {
                    alarm.enabled = !alarm.enabled;
                }
                self.save_config();
            }

            Message::SaveAlarm => {
                if let Some(edit) = &self.editing_alarm {
                    let time = chrono::NaiveTime::from_hms_opt(edit.hour.min(23), edit.minute.min(59), 0)
                        .unwrap_or_else(|| chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap());
                    let snooze_min = if edit.snooze_enabled { edit.snooze_minutes } else { 0 };
                    
                    if let Some(id) = edit.id {
                        if let Some(alarm) = self.alarms.iter_mut().find(|a| a.id == id) {
                            alarm.time = time;
                            alarm.label = edit.label.clone();
                            alarm.repeat_days = edit.repeat_days;
                            alarm.snooze_minutes = snooze_min;
                            alarm.sound = edit.sound.clone();
                        }
                    } else {
                        self.alarms.push(AlarmItem {
                            id: self.next_alarm_id,
                            time,
                            label: edit.label.clone(),
                            enabled: true,
                            repeat_days: edit.repeat_days,
                            snooze_minutes: snooze_min,
                            sound: edit.sound.clone(),
                        });
                        self.next_alarm_id += 1;
                        
                        let _ = notify_rust::Notification::new()
                            .summary("Alarm Set")
                            .body(&format!("⏰ Alarm set for {}", time.format("%H:%M")))
                            .icon("alarm-symbolic")
                            .timeout(notify_rust::Timeout::Milliseconds(2000))
                            .show();
                    }
                    
                    self.editing_alarm = None;
                    self.save_config();
                }
            }

            Message::CancelAlarmEdit => {
                self.editing_alarm = None;
            }

            Message::AlarmEditLabel(label) => {
                if let Some(edit) = &mut self.editing_alarm {
                    edit.label = label;
                }
            }

            Message::AlarmEditRepeatDay(day, enabled) => {
                if let Some(edit) = &mut self.editing_alarm {
                    if day < 7 {
                        edit.repeat_days[day as usize] = enabled;
                    }
                }
            }

            Message::AlarmEditEveryDay(enabled) => {
                if let Some(edit) = &mut self.editing_alarm {
                    for d in edit.repeat_days.iter_mut() {
                        *d = enabled;
                    }
                }
            }

            Message::AlarmEditSnoozeMinutes(minutes) => {
                if let Some(edit) = &mut self.editing_alarm {
                    edit.snooze_minutes = minutes.min(60);
                }
            }

            Message::AlarmEditSound(sound) => {
                if let Some(edit) = &mut self.editing_alarm {
                    edit.sound = sound;
                }
            }

            Message::AlarmEditHour(hour) => {
                if let Some(edit) = &mut self.editing_alarm {
                    edit.hour = hour.min(23);
                }
            }

            Message::AlarmEditMinute(minute) => {
                if let Some(edit) = &mut self.editing_alarm {
                    edit.minute = minute.min(59);
                }
            }

            Message::AlarmEditSnoozeEnabled(enabled) => {
                if let Some(edit) = &mut self.editing_alarm {
                    edit.snooze_enabled = enabled;
                }
            }

            Message::SnoozeAlarm(id) => {
                if let Some(alarm) = self.alarms.iter_mut().find(|a| a.id == id) {
                    // Snooze for the configured minutes
                    let _ = notify_rust::Notification::new()
                        .summary(&format!("😴 Snoozing {}", alarm.label))
                        .body(&format!("Will ring again in {} minutes", alarm.snooze_minutes))
                        .icon("alarm-symbolic")
                        .timeout(notify_rust::Timeout::Milliseconds(2000))
                        .show();
                }
            }

            // Timer editing messages
            Message::TimerSetSegment(timer_idx, segment, value) => {
                if let Some(timer) = self.timers.get_mut(timer_idx) {
                    if !timer.running {
                        match segment {
                            0 => timer.edit_hours = value.min(23),
                            1 => timer.edit_minutes = value.min(59),
                            2 => timer.edit_seconds = value.min(59),
                            _ => {}
                        }
                    }
                }
            }

            Message::TimerSetName(timer_idx, name) => {
                if let Some(timer) = self.timers.get_mut(timer_idx) {
                    timer.name = name;
                }
            }

            Message::TimerSelectSegment(timer_idx, segment) => {
                if let Some(timer) = self.timers.get_mut(timer_idx) {
                    if !timer.running {
                        timer.active_segment = segment.min(2);
                    }
                }
            }

            // World clock messages
            Message::AddWorldClock => {
                let cities = [
                    ("New York", "America/New_York"),
                    ("London", "Europe/London"),
                    ("Berlin", "Europe/Berlin"),
                    ("Tokyo", "Asia/Tokyo"),
                    ("Sydney", "Australia/Sydney"),
                    ("Dubai", "Asia/Dubai"),
                    ("Moscow", "Europe/Moscow"),
                    ("Shanghai", "Asia/Shanghai"),
                    ("Los Angeles", "America/Los_Angeles"),
                    ("Paris", "Europe/Paris"),
                ];
                let idx = (self.world_clocks.len() - 1) % cities.len();
                let (name, tz) = cities[idx];
                if let Ok(parsed_tz) = tz.parse::<chrono_tz::Tz>() {
                    let offset = parsed_tz.offset_from_utc_datetime(&chrono::Utc::now().naive_utc()).fix().local_minus_utc();
                    self.world_clocks.push(WorldClockItem {
                        name: name.to_string(),
                        country: String::new(),
                        timezone: tz.to_string(),
                        offset_hours: offset / 3600,
                        parsed_tz: Some(parsed_tz),
                    });
                    self.save_config();
                }
            }

            Message::DeleteWorldClock(index) => {
                if index < self.world_clocks.len() && self.world_clocks.len() > 1 {
                    self.world_clocks.remove(index);
                    self.save_config();
                }
            }

            Message::SelectTimezone(index, timezone) => {
                if index < self.world_clocks.len() {
                    if let Ok(parsed_tz) = timezone.parse::<chrono_tz::Tz>() {
                        let offset = parsed_tz.offset_from_utc_datetime(&chrono::Utc::now().naive_utc()).fix().local_minus_utc();
                        self.world_clocks[index].timezone = timezone;
                        self.world_clocks[index].offset_hours = offset / 3600;
                        self.world_clocks[index].parsed_tz = Some(parsed_tz);
                        self.save_config();
                    }
                }
            }

            Message::SearchCity(search) => {
                self.city_search = search;
            }

            Message::SelectSearchedCity(index) => {
                if index < CITIES.len() {
                    let (name, country, tz) = CITIES[index];
                    if let Ok(parsed_tz) = tz.parse::<chrono_tz::Tz>() {
                        let offset = parsed_tz.offset_from_utc_datetime(&chrono::Utc::now().naive_utc()).fix().local_minus_utc();
                        self.world_clocks.push(WorldClockItem {
                            name: name.to_string(),
                            country: country.to_string(),
                            timezone: tz.to_string(),
                            offset_hours: offset / 3600,
                            parsed_tz: Some(parsed_tz),
                        });
                        self.save_config();
                    }
                }
                self.show_city_picker = false;
                self.city_search.clear();
            }

            Message::ShowCityPicker => {
                self.show_city_picker = !self.show_city_picker;
                if !self.show_city_picker {
                    self.city_search.clear();
                }
            }

            Message::SortWorldClocks => {
                self.world_clocks.sort_by(|a, b| {
                    let a_is_local = a.timezone == "Local";
                    let b_is_local = b.timezone == "Local";
                    if a_is_local && !b_is_local {
                        return std::cmp::Ordering::Less;
                    }
                    if !a_is_local && b_is_local {
                        return std::cmp::Ordering::Greater;
                    }
                    a.offset_hours.cmp(&b.offset_hours)
                });
                self.save_config();
            }

            Message::MoveWorldClockUp(index) => {
                if index > 1 && index < self.world_clocks.len() {
                    self.world_clocks.swap(index, index - 1);
                    self.save_config();
                }
            }

            Message::MoveWorldClockDown(index) => {
                if index >= 1 && index < self.world_clocks.len() - 1 {
                    self.world_clocks.swap(index, index + 1);
                    self.save_config();
                }
            }

            Message::HoverClock(index) => {
                self.hovered_clock = index;
            }

            Message::NavTo(index) => {
                let nav_ids: Vec<_> = self.nav.iter().collect();
                if index < nav_ids.len() {
                    self.nav.activate(nav_ids[index]);
                    self.current_page = self.nav.data::<Page>(nav_ids[index]).cloned().unwrap_or_default();
                    return self.update_title();
                }
            }
        }
        Task::none()
    }

    /// Called when a nav item is selected.
    fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<cosmic::Action<Self::Message>> {
        self.nav.activate(id);
        self.current_page = self.nav.data::<Page>(id).cloned().unwrap_or_default();
        self.update_title()
    }
}

impl AppModel {
    /// Save current state to config
    fn save_config(&mut self) {
        if let Ok(context) = cosmic_config::Config::new("com.github.Moon-Mind.cosmic-watch", Config::VERSION) {
            let alarm_configs: Vec<config::AlarmConfig> = self.alarms.iter().map(|a| config::AlarmConfig {
                id: a.id,
                hour: a.time.hour(),
                minute: a.time.minute(),
                label: a.label.clone(),
                enabled: a.enabled,
                repeat_days: a.repeat_days,
                snooze_minutes: a.snooze_minutes,
                sound: a.sound.clone(),
            }).collect();
            
            let wc_configs: Vec<config::WorldClockConfig> = self.world_clocks.iter()
                .filter(|wc| wc.timezone != "Local")
                .map(|wc| config::WorldClockConfig {
                    name: wc.name.clone(),
                    country: wc.country.clone(),
                    timezone: wc.timezone.clone(),
                }).collect();
            
            let new_config = Config {
                alarms: alarm_configs,
                world_clocks: wc_configs,
                timer_presets: self.config.timer_presets.clone(),
            };
            let _ = new_config.write_entry(&context);
        }
    }

    /// Check if any alarms should trigger
    fn check_alarms(&mut self) {
        let current_time = self.current_time.time();
        
        for alarm in &self.alarms {
            if alarm.enabled && 
               alarm.time.hour() == current_time.hour() && 
               alarm.time.minute() == current_time.minute() &&
               current_time.second() == 0 { // Only trigger once per minute
                
                // Send notification
                notifications::send_alarm_notification(
                    &alarm.label,
                    &alarm.time.format("%H:%M").to_string()
                );
                
                println!("Alarm triggered: {} at {}", alarm.label, alarm.time.format("%H:%M"));
            }
        }
    }

    /// World Clock view — macOS-style card grid
    fn world_clock_view(&self) -> Element<'_, Message> {
        let cosmic_theme::Spacing { space_m, space_l, space_s, .. } = theme::active().cosmic().spacing;
        let clock_size = 130.0;

        // Build each clock card
        let mut grid_cards: Vec<Element<'_, Message>> = Vec::new();

        for (i, clock) in self.world_clocks.iter().enumerate() {
            let is_local = clock.timezone == "Local";

            let tz = clock.parsed_tz;
            let time_in_tz: chrono::NaiveTime = match (is_local, tz) {
                (true, _) => self.current_time.time(),
                (false, Some(tz)) => self.current_time.with_timezone(&tz).time(),
                (false, None) => self.current_time.time(),
            };

            let is_day = time_in_tz.hour() >= 6 && time_in_tz.hour() < 18;

            // Compute time difference with date-aware labels
            let date_local = self.current_time.date_naive();
            let date_target = match (is_local, tz) {
                (true, _) => date_local,
                (false, Some(tz)) => self.current_time.with_timezone(&tz).date_naive(),
                (false, None) => date_local,
            };
            let day_diff = (date_target - date_local).num_days();

            let local_offset_secs = self.current_time.offset().local_minus_utc();
            let tz_offset_secs = match (is_local, tz) {
                (true, _) => local_offset_secs,
                (false, Some(tz)) => tz.offset_from_utc_datetime(&self.current_time.naive_utc()).fix().local_minus_utc(),
                (false, None) => local_offset_secs,
            };
            let diff_secs = tz_offset_secs - local_offset_secs;
            let diff_hours = diff_secs.abs() / 3600;

            let diff_str = if diff_secs == 0 {
                fl!("today")
            } else if day_diff == 0 {
                if diff_secs > 0 {
                    format!("{}, {} {} ahead", fl!("today"), diff_hours, fl!("hours"))
                } else {
                    format!("{}, {} {} behind", fl!("today"), diff_hours, fl!("hours"))
                }
            } else if day_diff > 0 {
                    format!("{}, {} {} ahead", fl!("tomorrow"), diff_hours, fl!("hours"))
            } else {
                    format!("{}, {} {} behind", fl!("yesterday"), diff_hours, fl!("hours"))
            };

            let hovered = self.hovered_clock == Some(i);

            let clock_canvas = canvas::Canvas::<AnalogClock, Message, cosmic::Theme, cosmic::Renderer>::new(
                AnalogClock { time: time_in_tz, is_day }
            )
            .width(Length::Fixed(clock_size))
            .height(Length::Fixed(clock_size));

            // Reorder buttons below clock
            let mut reorder: Option<Element<'_, Message>> = None;
            if !is_local && i > 1 {
                let mut rr = widget::row().spacing(space_s);
                rr = rr.push(
                    widget::button::icon(widget::icon::from_name("go-up-symbolic"))
                        .on_press(Message::MoveWorldClockUp(i))
                        .width(Length::Shrink)
                        .padding(2)
                );
                if i < self.world_clocks.len() - 1 {
                    rr = rr.push(
                        widget::button::icon(widget::icon::from_name("go-down-symbolic"))
                            .on_press(Message::MoveWorldClockDown(i))
                            .width(Length::Shrink)
                            .padding(2)
                    );
                }
                reorder = Some(rr.into());
            }

            // Delete "X" on hover (top-right) — always same width to avoid layout shift
            let delete_w: f32 = 24.0;
            let delete_x: Element<'_, Message> = if !is_local && hovered {
                widget::container(
                    widget::button::icon(widget::icon::from_name("window-close-symbolic"))
                        .on_press(Message::DeleteWorldClock(i))
                        .width(Length::Fixed(delete_w))
                        .padding(0)
                )
                .width(Length::Fixed(delete_w))
                .padding(0)
                .into()
            } else {
                widget::horizontal_space()
                    .width(Length::Fixed(delete_w))
                    .into()
            };

            // Stack: clock canvas + delete spacer + info below
            let clock_section = widget::row()
                .push(
                    widget::column()
                        .push(
                            widget::row()
                                .push(
                                    widget::column()
                                        .push(clock_canvas)
                                        .width(Length::Fill)
                                        .align_x(Alignment::Center)
                                )
                                .push(delete_x)
                                .align_y(Alignment::Start)
                        )
                        .push(
                            widget::column()
                                .push(widget::text::body(&clock.name).size(13.0).width(Length::Fill))
                                .push(
                                    widget::text::caption(
                                        if clock.country.is_empty() { " " } else { &clock.country }
                                    ).size(10.0)
                                )
                                .push(widget::text::caption(diff_str.clone()).size(11.0))
                                .push_maybe(reorder)
                                .spacing(space_s)
                                .align_x(Alignment::Center)
                        )
                        .spacing(space_s)
                        .width(Length::Fill)
                        .align_x(Alignment::Center)
                )
                .spacing(space_s);

            let card = widget::mouse_area(
                widget::container(clock_section)
                    .padding(space_m)
                    .style(move |_: &theme::Theme| widget::container::Style {
                        background: Some(cosmic::iced::Background::Color(if is_day {
                            cosmic::iced::Color::from_rgba(1.0, 1.0, 1.0, 0.08)
                        } else {
                            cosmic::iced::Color::from_rgba(0.15, 0.15, 0.25, 0.15)
                        })),
                        border: cosmic::iced::Border {
                            radius: cosmic::iced::Radius::from(12.0),
                            width: 1.0,
                            color: if hovered {
                                cosmic::iced::Color::from_rgba(0.5, 0.5, 0.8, 0.5)
                            } else {
                                cosmic::iced::Color::from_rgba(0.5, 0.5, 0.5, 0.15)
                            },
                        },
                        ..Default::default()
                    })
            )
            .on_enter(Message::HoverClock(Some(i)))
            .on_exit(Message::HoverClock(None));

            let card_element: Element<'_, Message> = widget::container(card)
                .width(Length::Fill)
                .max_width(280.0)
                .into();

            grid_cards.push(card_element);
        }

        // Header: title on left, + and sort on right (macOS style)
        let header = widget::row()
            .push(widget::text::title2(fl!("world-clocks")).size(20.0))
            .push(widget::horizontal_space())
            .push(
                widget::button::icon(widget::icon::from_name("view-sort-ascending-symbolic"))
                    .on_press(Message::SortWorldClocks)
                    .width(Length::Shrink)
                    .padding(4)
            )
            .push(
                widget::button::icon(widget::icon::from_name("list-add-symbolic"))
                    .on_press(Message::ShowCityPicker)
                    .width(Length::Shrink)
                    .padding(4)
            )
            .spacing(space_s)
            .align_y(Vertical::Center);

        // Grid of cards using wrapping row
        let grid = widget::Row::with_children(grid_cards)
            .spacing(space_m)
            .wrap();

        let scrolled = widget::scrollable(
            widget::column()
                .push(
                    widget::container(
                        widget::column()
                            .push(header)
                            .align_x(Alignment::Center)
                            .spacing(space_m)
                    )
                    .width(Length::Fill)
                    .padding([space_l, space_l, 0, space_l])
                )
                .push(
                    widget::row()
                        .push(widget::horizontal_space())
                        .push(
                            widget::container(grid)
                                .max_width(900.0)
                        )
                        .push(widget::horizontal_space())
                        .padding([space_m, space_l, space_l, space_l])
                )
                .spacing(space_m)
        );

        if self.show_city_picker {
            let picker = self.city_picker_view();
            widget::column()
                .push(picker)
                .spacing(0)
                .into()
        } else {
            scrolled.into()
        }
    }

    fn city_picker_view(&self) -> Element<'_, Message> {
        let cosmic_theme::Spacing { space_m, space_l, space_s, .. } = theme::active().cosmic().spacing;

        let search_lower = self.city_search.to_lowercase();
        let filtered: Vec<&(&str, &str, &str)> = if search_lower.is_empty() {
            CITIES.iter().take(20).collect()
        } else {
            CITIES.iter()
                .filter(|(name, country, _)| {
                    name.to_lowercase().contains(&search_lower) ||
                    country.to_lowercase().contains(&search_lower)
                })
                .take(50)
                .collect()
        };

        let mut list = widget::column().spacing(space_s);

        for &&(name, country, _tz) in filtered.iter() {
            let entry = widget::row()
                .push(widget::text::body(name).width(Length::FillPortion(1)))
                .push(widget::text::caption(country).width(Length::FillPortion(1)))
                .push(
                    widget::button::icon(widget::icon::from_name("list-add-symbolic"))
                        .on_press(Message::SelectSearchedCity(
                            CITIES.iter().position(|c| c.0 == name).unwrap_or(0)
                        ))
                        .width(Length::Shrink)
                        .padding(4)
                )
                .spacing(space_s)
                .align_y(Vertical::Center);

            list = list.push(entry);
        }

        widget::column()
            .push(
                widget::row()
                    .push(
                        widget::text_input("Search cities", &self.city_search)
                            .on_input(Message::SearchCity)
                            .width(Length::Fill)
                    )
                    .push(
                        widget::button::standard(fl!("cancel"))
                            .on_press(Message::ShowCityPicker)
                            .width(Length::Shrink)
                    )
                    .spacing(space_s)
                    .align_y(Vertical::Center)
            )
            .push(
                widget::scrollable(list)
                    .height(Length::Fill)
            )
            .spacing(space_m)
            .padding(space_l)
            .into()
    }

    /// Alarm view — macOS-style card list with overlay editing
    fn alarm_view(&self) -> Element<'_, Message> {
        let cosmic_theme::Spacing { space_m, space_l, space_s, .. } = theme::active().cosmic().spacing;

        // Header with plus button
        let header = widget::row()
            .push(widget::text::title1("⏰").size(28.0))
            .push(widget::text::title2(fl!("alarms")).size(20.0))
            .push(widget::horizontal_space())
            .push(
                widget::button::icon(widget::icon::from_name("list-add-symbolic"))
                    .on_press(Message::AddAlarm)
                    .width(Length::Shrink)
                    .padding(4)
            )
            .spacing(space_s)
            .align_y(Vertical::Center);

        let mut list = widget::column().spacing(space_m);

        if self.alarms.is_empty() {
            list = list.push(
                widget::text::body(fl!("no-alarms"))
                    .apply(widget::container)
                    .width(Length::Fill)
                    .padding(space_l)
            );
        } else {
            for alarm in &self.alarms {
                let time_str = alarm.time.format("%H:%M").to_string();

                // Repeat summary
                let all_on = alarm.repeat_days.iter().all(|&d| d);
                let repeat_str = if all_on {
                    fl!("every-day")
                } else {
                    let mut days = Vec::new();
                    for (i, &on) in alarm.repeat_days.iter().enumerate() {
                        if on {
                            days.push(DAY_LABELS[i]);
                        }
                    }
                    days.join(" ")
                };

                let card = widget::mouse_area(
                    widget::row()
                        .push(
                            widget::column()
                                .push(
                                    widget::text::title1(time_str).size(32.0)
                                )
                                .push(
                                    widget::text::body(&alarm.label).size(14.0)
                                )
                                .push(
                                    widget::text::caption(if all_on { format!("{} — {}", repeat_str, alarm.sound) } else { format!("{} — {}", repeat_str, alarm.sound) }).size(10.0)
                                )
                                .spacing(space_s / 2)
                                .width(Length::Fill)
                        )
                        .push(
                            widget::toggler(alarm.enabled)
                                .on_toggle(move |_| Message::ToggleAlarm(alarm.id))
                        )
                        .spacing(space_m)
                        .align_y(Vertical::Center)
                        .apply(widget::container)
                        .padding(space_m)
                        .width(Length::Fill)
                        .style(|_: &theme::Theme| widget::container::Style {
                            background: Some(cosmic::iced::Background::Color(
                                cosmic::iced::Color::from_rgba(0.5, 0.5, 0.5, 0.06)
                            )),
                            border: cosmic::iced::Border {
                                radius: cosmic::iced::Radius::from(10.0),
                                width: 1.0,
                                color: cosmic::iced::Color::from_rgba(0.5, 0.5, 0.5, 0.12),
                            },
                            ..Default::default()
                        })
                )
                .on_press(Message::EditAlarm(alarm.id));

                list = list.push(card);
            }
        }

        let content = widget::column()
            .push(
                widget::container(header)
                    .width(Length::Fill)
                    .padding([space_l, space_l, 0, space_l])
            )
            .push(
                widget::scrollable(
                    widget::container(list)
                        .width(Length::Fill)
                        .padding([space_m, space_l, space_l, space_l])
                )
                .width(Length::Fill)
            )
            .spacing(space_s);

        if self.editing_alarm.is_some() {
            // Show just the edit overlay
            if let Some(edit) = &self.editing_alarm {
                self.alarm_edit_overlay(edit)
            } else {
                unreachable!()
            }
        } else {
            content
                .apply(widget::container)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        }
    }

    /// macOS-style alarm edit overlay card
    fn alarm_edit_overlay<'a>(&'a self, edit: &'a AlarmEdit) -> Element<'a, Message> {
        let cosmic_theme::Spacing { space_m, space_l, space_s, .. } = theme::active().cosmic().spacing;
        // Time display with up/down buttons
        let btn_sz = 20.0;
        let up = |msg| {
            widget::button::icon(widget::icon::from_name("go-up-symbolic"))
                .on_press(msg)
                .width(Length::Fixed(btn_sz))
                .padding(0)
        };
        let dn = |msg| {
            widget::button::icon(widget::icon::from_name("go-down-symbolic"))
                .on_press(msg)
                .width(Length::Fixed(btn_sz))
                .padding(0)
        };

        let hour_edit = widget::column()
            .push(up(Message::AlarmEditHour(if edit.hour == 23 { 0 } else { edit.hour + 1 })))
            .push(widget::text::title1(format!("{:02}", edit.hour)).size(40.0).align_x(Alignment::Center))
            .push(dn(Message::AlarmEditHour(if edit.hour == 0 { 23 } else { edit.hour - 1 })))
            .spacing(space_s / 2)
            .align_x(Alignment::Center);

        let min_edit = widget::column()
            .push(up(Message::AlarmEditMinute(if edit.minute == 59 { 0 } else { edit.minute + 1 })))
            .push(widget::text::title1(format!("{:02}", edit.minute)).size(40.0).align_x(Alignment::Center))
            .push(dn(Message::AlarmEditMinute(if edit.minute == 0 { 59 } else { edit.minute - 1 })))
            .spacing(space_s / 2)
            .align_x(Alignment::Center);

        let time_section = widget::column()
            .push(
                widget::row()
                    .push(widget::horizontal_space())
                    .push(hour_edit)
                    .push(widget::text::title1(":").size(40.0).align_y(Vertical::Center))
                    .push(min_edit)
                    .push(widget::horizontal_space())
                    .spacing(space_s)
                    .align_y(Vertical::Center)
            )
            .spacing(space_s)
            .align_x(Alignment::Center);

        // Repeat day buttons
        let all_on = edit.repeat_days.iter().all(|&d| d);
        let repeat_summary = if all_on {
            fl!("every-day")
        } else {
            let mut days = Vec::new();
            for (i, &on) in edit.repeat_days.iter().enumerate() {
                if on {
                    days.push(DAY_LABELS[i]);
                }
            }
            if days.is_empty() {
                String::from("—")
            } else {
                days.join(" ")
            }
        };

        // "Every Day" quick toggle
        let all_on = edit.repeat_days.iter().all(|&d| d);
        let every_day = widget::mouse_area(
            widget::container(
                widget::text::body(fl!("every-day")).size(13.0)
                    .apply(widget::container)
                    .padding([space_s, space_m])
            )
            .style(move |_: &theme::Theme| widget::container::Style {
                background: Some(cosmic::iced::Background::Color(
                    if all_on {
                        cosmic::iced::Color::from_rgb8(0xFF, 0x9F, 0x0A)
                    } else {
                        cosmic::iced::Color::from_rgba(0.5, 0.5, 0.5, 0.15)
                    }
                )),
                border: cosmic::iced::Border {
                    radius: cosmic::iced::Radius::from(20.0),
                    width: 0.0,
                    color: cosmic::iced::Color::TRANSPARENT,
                },
                ..Default::default()
            })
        )
        .on_press(Message::AlarmEditEveryDay(!all_on));

        let mut day_row = widget::row().spacing(space_s).align_y(Vertical::Center);
        for (i, &label) in DAY_LABELS.iter().enumerate() {
            let is_on = edit.repeat_days[i];
            let day_btn = widget::mouse_area(
                widget::container(
                    widget::text::body(label).size(13.0)
                        .apply(widget::container)
                        .padding([space_s + 4, space_s + 2])
                )
                .style(move |_: &theme::Theme| widget::container::Style {
                    background: Some(cosmic::iced::Background::Color(
                        if is_on {
                            cosmic::iced::Color::from_rgb8(0xFF, 0x9F, 0x0A)
                        } else {
                            cosmic::iced::Color::from_rgba(0.5, 0.5, 0.5, 0.15)
                        }
                    )),
                    border: cosmic::iced::Border {
                        radius: cosmic::iced::Radius::from(20.0),
                        width: 0.0,
                        color: cosmic::iced::Color::TRANSPARENT,
                    },
                    ..Default::default()
                })
            )
            .on_press(Message::AlarmEditRepeatDay(i as u8, !is_on));

            day_row = day_row.push(day_btn);
        }

        let repeat_section = widget::column()
            .push(widget::text::caption(fl!("repeat")).size(12.0))
            .push(
                widget::scrollable::horizontal(
                    widget::row()
                        .push(every_day)
                        .push(day_row)
                        .spacing(space_s)
                        .align_y(Vertical::Center)
                )
                .width(Length::Fill)
                .height(Length::Fixed(48.0))
            )
            .push(widget::text::caption(repeat_summary.clone()).size(11.0))
            .spacing(space_s);

        // Label input
        let label_input = widget::text_input(fl!("alarm-label"), &edit.label)
            .on_input(Message::AlarmEditLabel);

        // Sound selector (dropdown)
        let sound_index = ALARM_SOUNDS.iter().position(|&s| s == edit.sound);
        let sound_dropdown = widget::dropdown::dropdown(
            Cow::Borrowed(ALARM_SOUNDS),
            sound_index,
            |i| Message::AlarmEditSound(ALARM_SOUNDS[i].to_string()),
        );

        let sound_section = widget::column()
            .push(widget::text::caption(fl!("sound")).size(12.0))
            .push(sound_dropdown)
            .spacing(space_s);

        // Snooze checkbox
        let snooze_check = widget::checkbox(fl!("snooze"), edit.snooze_enabled)
            .on_toggle(Message::AlarmEditSnoozeEnabled);

        // Snooze duration (dropdown, only shown when snooze enabled)
        let snooze_index = SNOOZE_OPTIONS.iter().position(|&s| s == edit.snooze_minutes);
        let snooze_duration_dropdown = widget::dropdown::dropdown(
            Cow::Borrowed(SNOOZE_STR),
            snooze_index,
            |i| Message::AlarmEditSnoozeMinutes(SNOOZE_OPTIONS[i]),
        );

        let snooze_duration = if edit.snooze_enabled {
            widget::column()
                .push(snooze_check)
                .push(
                    widget::row()
                        .push(widget::horizontal_space().width(Length::Fixed(40.0)))
                        .push(
                            widget::column()
                                .push(widget::text::caption(fl!("snooze-duration")).size(12.0))
                                .push(snooze_duration_dropdown)
                                .spacing(space_s)
                        )
                        .spacing(space_s)
                )
                .spacing(space_s)
        } else {
            widget::column()
                .push(snooze_check)
                .spacing(space_s)
        };

        // Bottom buttons
        let is_new = edit.id.is_none();
        let delete_btn = if !is_new {
            widget::button::destructive(fl!("delete-alarm"))
                .on_press_maybe(edit.id.map(Message::DeleteAlarm))
                .width(Length::Shrink)
        } else {
            widget::button::destructive(fl!("delete-alarm"))
                .width(Length::Shrink)
        };

        let buttons = widget::row()
            .push(delete_btn)
            .push(widget::horizontal_space())
            .push(
                widget::button::standard(fl!("cancel"))
                    .on_press(Message::CancelAlarmEdit)
                    .width(Length::Shrink)
            )
            .push(
                widget::button::suggested(fl!("save-alarm"))
                    .on_press(Message::SaveAlarm)
                    .width(Length::Shrink)
            )
            .spacing(space_m)
            .align_y(Vertical::Center);

        // Assemble the edit card
        let card = widget::column()
            .push(time_section)
            .push(widget::divider::horizontal::default())
            .push(repeat_section)
            .push(widget::divider::horizontal::default())
            .push(label_input)
            .push(widget::divider::horizontal::default())
            .push(sound_section)
            .push(widget::divider::horizontal::default())
            .push(snooze_duration)
            .push(widget::divider::horizontal::default())
            .push(buttons)
            .spacing(space_m)
            .width(Length::Fill);

        let edit_card = widget::container(card)
            .width(Length::Fixed(360.0))
            .padding(space_l)
            .style(|theme: &theme::Theme| -> widget::container::Style {
                let is_dark = theme.cosmic().is_dark;
                let bg = if is_dark {
                    cosmic::iced::Color::from_rgb8(0x2B, 0x2B, 0x2B)
                } else {
                    cosmic::iced::Color::from_rgb8(0xF5, 0xF5, 0xF5)
                };
                widget::container::Style {
                    background: Some(cosmic::iced::Background::Color(bg)),
                    border: cosmic::iced::Border {
                        radius: cosmic::iced::Radius::from(14.0),
                        width: 1.0,
                        color: cosmic::iced::Color::from_rgba(0.5, 0.5, 0.5, 0.3),
                    },
                    shadow: cosmic::iced::Shadow {
                        color: cosmic::iced::Color::from_rgba(0.0, 0.0, 0.0, 0.3),
                        offset: cosmic::iced::Vector::new(0.0, 4.0),
                        blur_radius: 12.0,
                    },
                    ..Default::default()
                }
            });

        // Center the edit card in a scrollable page so it's never cut off
        widget::scrollable(
            widget::column()
                .push(widget::horizontal_space().height(Length::Fixed(space_l as f32)))
                .push(
                    widget::row()
                        .push(widget::horizontal_space())
                        .push(edit_card)
                        .push(widget::horizontal_space())
                )
                .push(widget::horizontal_space())
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
    fn stopwatch_view(&self) -> Element<'_, Message> {
        let cosmic_theme::Spacing { space_m, space_l, space_s, .. } = theme::active().cosmic().spacing;

        // Time display
        let display = widget::text::title1(&self.stopwatch_str)
            .size(56.0)
            .align_x(Alignment::Center);

        // Lap table
        let mut lap_rows = widget::column().spacing(space_s);
        let is_running = self.stopwatch_running && !self.stopwatch_paused;

        // Header row
        let header_row = widget::row()
            .push(widget::text::caption(fl!("lap")).width(Length::Fixed(50.0)).size(10.0))
            .push(widget::text::caption("Split").width(Length::Fill).size(10.0))
            .push(widget::text::caption("Total").width(Length::Fill).size(10.0))
            .spacing(space_s);
        lap_rows = lap_rows.push(
            widget::container(header_row)
                .padding([0, space_s])
                .width(Length::Fill)
        );

        if self.stopwatch_laps.is_empty() {
            lap_rows = lap_rows.push(
                widget::text::caption(fl!("no-laps"))
                    .apply(widget::container)
                    .padding(space_m)
                    .width(Length::Fill)
            );
        } else {
            for (i, lap) in self.stopwatch_laps.iter().rev().enumerate() {
                let lap_num = self.stopwatch_laps.len() - i;
                let split_str = format!("{:02}:{:02}.{:02}",
                    lap.split.as_secs() / 60,
                    lap.split.as_secs() % 60,
                    (lap.split.as_millis() % 1000) / 10
                );
                let total_str = format!("{:02}:{:02}.{:02}",
                    lap.total.as_secs() / 60,
                    lap.total.as_secs() % 60,
                    (lap.total.as_millis() % 1000) / 10
                );

                let row = widget::row()
                    .push(widget::text::body(format!("Lap {}", lap_num)).width(Length::Fixed(50.0)))
                    .push(widget::text::body(split_str).width(Length::Fill))
                    .push(widget::text::body(total_str).width(Length::Fill))
                    .spacing(space_s);

                lap_rows = lap_rows.push(
                    widget::container(row)
                        .padding([space_s / 2, space_s])
                        .width(Length::Fill)
                );
            }
        }

        // Buttons
        let start_btn = widget::button::suggested(
            if is_running { fl!("stop") } else { fl!("start") }
        )
        .on_press(if is_running { Message::StopStopwatch } else { Message::StartStopwatch });

        let secondary_btn = if is_running {
            widget::button::standard(fl!("lap"))
                .on_press(Message::RecordLap)
        } else {
            widget::button::standard(fl!("reset"))
                .on_press(Message::ResetStopwatch)
        };

        let buttons = widget::row()
            .push(secondary_btn)
            .push(widget::horizontal_space())
            .push(start_btn)
            .spacing(space_m)
            .align_y(Vertical::Center);

        let content = widget::column()
            .push(widget::horizontal_space())
            .push(display)
            .push(widget::horizontal_space().height(Length::Fixed(space_l as f32)))
            .push(
                widget::scrollable(lap_rows)
                    .width(Length::Fill)
                    .height(Length::Fill)
            )
            .push(buttons)
            .spacing(space_m)
            .align_x(Alignment::Center);

        let centered = widget::row()
            .push(widget::horizontal_space())
            .push(
                widget::container(content)
                    .width(Length::Fill)
                    .max_width(400.0)
            )
            .push(widget::horizontal_space());

        centered
            .apply(widget::container)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(space_l)
            .into()
    }

    /// macOS-style timer view with multiple timers
    fn timer_view(&self) -> Element<'_, Message> {
        let cosmic_theme::Spacing { space_m, space_l, .. } = theme::active().cosmic().spacing;

        // Header with plus button
        let header = widget::row()
            .push(widget::horizontal_space())
            .push(
                widget::button::icon(widget::icon::from_name("list-add-symbolic"))
                    .on_press(Message::AddTimer)
                    .width(Length::Shrink)
                    .padding(4)
            );

        let mut content = widget::column()
            .push(
                widget::container(header)
                    .width(Length::Fill)
                    .padding([space_l, space_l, 0, space_l])
            )
            .spacing(space_m)
            .align_x(Alignment::Center);

        if self.timers.is_empty() {
            content = content.push(
                widget::text::body("No timers").align_x(Alignment::Center)
                    .apply(widget::container)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center)
            );
        } else {
            for (i, timer) in self.timers.iter().enumerate() {
                let card = self.timer_card(i, timer);
                content = content.push(card);
            }
        }

        content
            .apply(widget::container)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding([0, 0, space_l, 0])
            .into()
    }

    /// Card for a single timer
    fn timer_card<'a>(&'a self, index: usize, timer: &'a TimerItem) -> Element<'a, Message> {
        let cosmic_theme::Spacing { space_m, space_l, space_s, .. } = theme::active().cosmic().spacing;

        let is_completed = timer.remaining == Duration::default() && !timer.running;
        let is_setting = !timer.running && !is_completed;
        let is_active = timer.running && !timer.paused;

        // Time display
        let time_display: Element<'a, Message> = if is_setting {
            // Editable segmented display (macOS style)
            let hours_str = format!("{:02}", timer.edit_hours);
            let minutes_str = format!("{:02}", timer.edit_minutes);
            let seconds_str = format!("{:02}", timer.edit_seconds);

            let input_width = Length::Fixed(70.0);
            let input_size = 48.0;

            let hour_col = widget::column()
                .push(
                    widget::mouse_area(
                        widget::container(
                            widget::text_input("00", hours_str)
                                .on_input(move |s| {
                                    let v = s.chars().filter(|c| c.is_ascii_digit()).take(2).collect::<String>();
                                    Message::TimerSetSegment(index, 0, v.parse().unwrap_or(0))
                                })
                                .width(input_width)
                                .size(input_size)
                        )
                        .style(move |_: &theme::Theme| {
                            let bg = if timer.active_segment == 0 {
                                cosmic::iced::Color::from_rgba(1.0, 0.6, 0.0, 0.25)
                            } else {
                                cosmic::iced::Color::from_rgba(0.5, 0.5, 0.5, 0.08)
                            };
                            widget::container::Style {
                                background: Some(cosmic::iced::Background::Color(bg)),
                                border: cosmic::iced::Border {
                                    radius: cosmic::iced::Radius::from(8.0),
                                    width: 0.0,
                                    color: cosmic::iced::Color::TRANSPARENT,
                                },
                                ..Default::default()
                            }
                        })
                    )
                    .on_press(Message::TimerSelectSegment(index, 0))
                )
                .push(widget::text::caption(fl!("hr")).size(11.0).align_x(Alignment::Center))
                .spacing(space_s / 2)
                .align_x(Alignment::Center);

            let min_col = widget::column()
                .push(
                    widget::mouse_area(
                        widget::container(
                            widget::text_input("00", minutes_str)
                                .on_input(move |s| {
                                    let v = s.chars().filter(|c| c.is_ascii_digit()).take(2).collect::<String>();
                                    Message::TimerSetSegment(index, 1, v.parse().unwrap_or(0))
                                })
                                .width(input_width)
                                .size(input_size)
                        )
                        .style(move |_: &theme::Theme| {
                            let bg = if timer.active_segment == 1 {
                                cosmic::iced::Color::from_rgba(1.0, 0.6, 0.0, 0.25)
                            } else {
                                cosmic::iced::Color::from_rgba(0.5, 0.5, 0.5, 0.08)
                            };
                            widget::container::Style {
                                background: Some(cosmic::iced::Background::Color(bg)),
                                border: cosmic::iced::Border {
                                    radius: cosmic::iced::Radius::from(8.0),
                                    width: 0.0,
                                    color: cosmic::iced::Color::TRANSPARENT,
                                },
                                ..Default::default()
                            }
                        })
                    )
                    .on_press(Message::TimerSelectSegment(index, 1))
                )
                .push(widget::text::caption(fl!("min")).size(11.0).align_x(Alignment::Center))
                .spacing(space_s / 2)
                .align_x(Alignment::Center);

            let sec_col = widget::column()
                .push(
                    widget::mouse_area(
                        widget::container(
                            widget::text_input("00", seconds_str)
                                .on_input(move |s| {
                                    let v = s.chars().filter(|c| c.is_ascii_digit()).take(2).collect::<String>();
                                    Message::TimerSetSegment(index, 2, v.parse().unwrap_or(0))
                                })
                                .width(input_width)
                                .size(input_size)
                        )
                        .style(move |_: &theme::Theme| {
                            let bg = if timer.active_segment == 2 {
                                cosmic::iced::Color::from_rgba(1.0, 0.6, 0.0, 0.25)
                            } else {
                                cosmic::iced::Color::from_rgba(0.5, 0.5, 0.5, 0.08)
                            };
                            widget::container::Style {
                                background: Some(cosmic::iced::Background::Color(bg)),
                                border: cosmic::iced::Border {
                                    radius: cosmic::iced::Radius::from(8.0),
                                    width: 0.0,
                                    color: cosmic::iced::Color::TRANSPARENT,
                                },
                                ..Default::default()
                            }
                        })
                    )
                    .on_press(Message::TimerSelectSegment(index, 2))
                )
                .push(widget::text::caption(fl!("sec")).size(11.0).align_x(Alignment::Center))
                .spacing(space_s / 2)
                .align_x(Alignment::Center);

            widget::row()
                .push(widget::horizontal_space())
                .push(hour_col)
                .push(widget::text::title1(":").size(input_size).align_y(Vertical::Center))
                .push(min_col)
                .push(widget::text::title1(":").size(input_size).align_y(Vertical::Center))
                .push(sec_col)
                .push(widget::horizontal_space())
                .align_y(Vertical::Center)
                .spacing(space_s)
                .into()
        } else {
            // Non-editable countdown display
            let total_secs = timer.remaining.as_secs();
            let time_str = format!("{:02}:{:02}:{:02}",
                total_secs / 3600,
                (total_secs % 3600) / 60,
                total_secs % 60
            );

            widget::row()
                .push(widget::horizontal_space())
                .push(widget::text::title1(time_str).size(56.0))
                .push(widget::horizontal_space())
                .align_y(Vertical::Center)
                .into()
        };

        // Name input / display
        let name_widget: Element<'_, Message> = if is_setting {
            widget::text_input(fl!("timer-label"), &timer.name)
                .on_input(move |s| Message::TimerSetName(index, s))
                .width(Length::Fixed(200.0))
                .into()
        } else {
            if timer.name.is_empty() {
                widget::text::body("Timer").size(14.0).into()
            } else {
                widget::text::body(&timer.name).size(14.0).into()
            }
        };

        // Bottom buttons
        let buttons = if is_setting {
            // Cancel + Start
            widget::row()
                .push(
                    widget::button::standard(fl!("cancel"))
                        .on_press(Message::DeleteTimer(index))
                        .width(Length::Fixed(100.0))
                )
                .push(
                    widget::button::suggested(fl!("start"))
                        .on_press(Message::TimerStart(index))
                        .width(Length::Fixed(100.0))
                )
                .spacing(space_m)
                .align_y(Vertical::Center)
        } else if is_completed {
            // Reset
            widget::row()
                .push(
                    widget::button::standard(fl!("reset"))
                        .on_press(Message::TimerReset(index))
                        .width(Length::Fixed(100.0))
                )
                .push(
                    widget::button::destructive("Delete")
                        .on_press(Message::DeleteTimer(index))
                        .width(Length::Fixed(100.0))
                )
                .spacing(space_m)
                .align_y(Vertical::Center)
        } else {
            // Running or paused: Pause/Resume + Cancel
            let pause_resume = if is_active {
                widget::button::standard(fl!("pause"))
                    .on_press(Message::TimerPause(index))
                    .width(Length::Fixed(100.0))
            } else {
                // paused
                widget::button::suggested(fl!("resume"))
                    .on_press(Message::TimerPause(index))
                    .width(Length::Fixed(100.0))
            };

            widget::row()
                .push(pause_resume)
                .push(
                    widget::button::standard(fl!("cancel"))
                        .on_press(Message::TimerStop(index))
                        .width(Length::Fixed(100.0))
                )
                .spacing(space_m)
                .align_y(Vertical::Center)
        };

        // Assemble card
        let card = widget::column()
            .push(time_display)
            .push(
                widget::container(name_widget)
                    .align_x(Horizontal::Center)
                    .width(Length::Fill)
            )
            .push(
                widget::container(buttons)
                    .align_x(Horizontal::Center)
                    .width(Length::Fill)
            )
            .spacing(space_m)
            .align_x(Alignment::Center);

        widget::container(card)
            .width(Length::Fill)
            .max_width(400.0)
            .padding(space_l)
            .style(move |_: &theme::Theme| widget::container::Style {
                background: Some(cosmic::iced::Background::Color(
                    cosmic::iced::Color::from_rgba(0.5, 0.5, 0.5, 0.08)
                )),
                border: cosmic::iced::Border {
                    radius: cosmic::iced::Radius::from(12.0),
                    width: 1.0,
                    color: cosmic::iced::Color::from_rgba(0.5, 0.5, 0.5, 0.15),
                },
                ..Default::default()
            })
            .into()
    }

    /// The about page for this app.
    pub fn about(&self) -> Element<'_, Message> {
        let cosmic_theme::Spacing { space_xxs, .. } = theme::active().cosmic().spacing;

        let icon = widget::svg(widget::svg::Handle::from_memory(APP_ICON));
        let title = widget::text::title3(fl!("app-title"));

        let hash = env!("VERGEN_GIT_SHA");
        let short_hash: String = hash.chars().take(7).collect();
        let date = env!("VERGEN_GIT_COMMIT_DATE");

        let link = widget::button::link(REPOSITORY)
            .on_press(Message::OpenRepositoryUrl)
            .padding(0);

        widget::column()
            .push(icon)
            .push(title)
            .push(link)
            .push(
                widget::button::link(fl!(
                    "git-description",
                    hash = short_hash.as_str(),
                    date = date
                ))
                .on_press(Message::LaunchUrl(format!("{REPOSITORY}/commits/{hash}")))
                .padding(0),
            )
            .align_x(Alignment::Center)
            .spacing(space_xxs)
            .into()
    }

    /// Updates the header and window titles.
    pub fn update_title(&mut self) -> Task<cosmic::Action<Message>> {
        let mut window_title = fl!("app-title");

        if let Some(page) = self.nav.text(self.nav.active()) {
            window_title.push_str(" — ");
            window_title.push_str(page);
        }

        if let Some(id) = self.core.main_window_id() {
            self.set_window_title(window_title, id)
        } else {
            Task::none()
        }
    }
}

/// The page to display in the application.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Page {
    #[default]
    WorldClock,
    Alarm,
    Stopwatch,
    Timer,
}

/// The context page to display in the context drawer.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum ContextPage {
    #[default]
    About,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuAction {
    About,
}

impl menu::action::MenuAction for MenuAction {
    type Message = Message;

    fn message(&self) -> Self::Message {
        match self {
            MenuAction::About => Message::ToggleContextPage(ContextPage::About),
        }
    }
}

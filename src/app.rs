// SPDX-License-Identifier: MPL-2.0

use crate::config::Config;
use crate::fl;
use crate::notifications;
use chrono::Timelike;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, Length, Subscription};
use cosmic::prelude::*;
use cosmic::widget::{self, icon, menu, nav_bar};
use cosmic::{cosmic_theme, theme};
use std::collections::HashMap;
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
    /// Key bindings for the application's menu bar.
    key_binds: HashMap<menu::KeyBind, MenuAction>,
    /// Configuration data that persists between application runs.
    config: Config,
    /// Current time for display
    current_time: chrono::DateTime<chrono::Local>,
    /// Stopwatch state
    stopwatch_time: Duration,
    stopwatch_running: bool,
    /// Timer state
    timer_duration: Duration,
    timer_remaining: Duration,
    timer_running: bool,
    /// Alarm state
    alarms: Vec<AlarmItem>,
    next_alarm_id: u32,
    /// Alarm editing state
    editing_alarm: Option<AlarmEdit>,
    /// World clock timezones
    world_clocks: Vec<WorldClockItem>,
    /// Stopwatch lap times
    lap_times: Vec<Duration>,
    /// Timer editing state
    editing_timer: bool,
    timer_edit_hours: u32,
    timer_edit_minutes: u32,
    timer_edit_seconds: u32,
    /// Animation state
    pulse_animation: f32,
    animation_direction: i8,
}

#[derive(Clone, Debug)]
pub struct AlarmItem {
    pub id: u32,
    pub time: chrono::NaiveTime,
    pub label: String,
    pub enabled: bool,
    #[allow(dead_code)]
    pub repeat_days: [bool; 7],
    #[allow(dead_code)]
    pub snooze_minutes: u32,
}

#[derive(Clone, Debug)]
pub struct AlarmEdit {
    pub id: Option<u32>, // None for new alarm
    pub hour: u32,
    pub minute: u32,
    pub label: String,
    pub repeat_days: [bool; 7], // Mon-Sun
    pub snooze_minutes: u32,
}

#[derive(Clone, Debug)]
pub struct WorldClockItem {
    pub name: String,
    pub timezone: String,
    pub offset_hours: i32,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    OpenRepositoryUrl,
    ToggleContextPage(ContextPage),
    UpdateConfig(Config),
    LaunchUrl(String),
    UpdateTime,
    // Stopwatch messages
    StartStopwatch,
    StopStopwatch,
    ResetStopwatch,
    // Timer messages
    StartTimer,
    StopTimer,
    ResetTimer,
    // Timer messages
    SetTimerHours(u32),
    SetTimerMinutes(u32),
    SetTimerSeconds(u32),
    StartTimerEdit,
    SaveTimerEdit,
    CancelTimerEdit,
    // Alarm messages
    AddAlarm,
    EditAlarm(u32),
    DeleteAlarm(u32),
    ToggleAlarm(u32),
    SaveAlarm,
    CancelAlarmEdit,
    AlarmEditHour(u32),
    AlarmEditMinute(u32),
    AlarmEditLabel(String),
    #[allow(dead_code)]
    AlarmEditRepeatDay(u8, bool),
    #[allow(dead_code)]
    AlarmEditSnoozeMinutes(u32),
    #[allow(dead_code)]
    SnoozeAlarm(u32),
    // World clock messages
    AddWorldClock,
    #[allow(dead_code)]
    DeleteWorldClock(usize),
    // Stopwatch messages
    RecordLap,
    ClearLaps,
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
            .icon(icon::from_name("chronometer-symbolic"));

        nav.insert()
            .text(fl!("timer"))
            .data::<Page>(Page::Timer)
            .icon(icon::from_name("timer-symbolic"));

        // Construct the app model with the runtime's core.
        let mut app = AppModel {
            core,
            context_page: ContextPage::default(),
            nav,
            key_binds: HashMap::new(),
            config: cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
                .map(|context| match Config::get_entry(&context) {
                    Ok(config) => config,
                    Err((_errors, config)) => config,
                })
                .unwrap_or_default(),
            current_time: chrono::Local::now(),
            stopwatch_time: Duration::default(),
            stopwatch_running: false,
            timer_duration: Duration::from_secs(300), // 5 minutes default
            timer_remaining: Duration::from_secs(300),
            timer_running: false,
            alarms: Vec::new(),
            next_alarm_id: 1,
            editing_alarm: None,
            world_clocks: vec![
                WorldClockItem {
                    name: String::from("Local"),
                    timezone: String::from("Local"),
                    offset_hours: 0,
                },
            ],
            lap_times: Vec::new(),
            editing_timer: false,
            timer_edit_hours: 0,
            timer_edit_minutes: 5,
            timer_edit_seconds: 0,
            pulse_animation: 1.0,
            animation_direction: 1,
        };

        let command = app.update_title();

        (app, command)
    }

    /// Elements to pack at the start of the header bar.
    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        let menu_bar = menu::bar(vec![menu::Tree::with_children(
            menu::root(fl!("view")).apply(Element::from),
            menu::items(
                &self.key_binds,
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
        ];

        // Add more frequent updates for stopwatch and timer
        if self.stopwatch_running || self.timer_running {
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

            Message::LaunchUrl(url) => match open::that_detached(&url) {
                Ok(()) => {}
                Err(err) => {
                    eprintln!("failed to open {url:?}: {err}");
                }
            },

            Message::UpdateTime => {
                self.current_time = chrono::Local::now();
                
                if self.stopwatch_running {
                    self.stopwatch_time += Duration::from_millis(100);
                }
                
                if self.timer_running && self.timer_remaining > Duration::default() {
                    self.timer_remaining = self.timer_remaining.saturating_sub(Duration::from_millis(100));
                    if self.timer_remaining == Duration::default() {
                        self.timer_running = false;
                        // Timer finished - send notification
                        notifications::send_timer_notification();
                    }
                }

                // Update pulse animation
                self.pulse_animation += 0.02 * self.animation_direction as f32;
                if self.pulse_animation >= 1.2 {
                    self.animation_direction = -1;
                } else if self.pulse_animation <= 0.8 {
                    self.animation_direction = 1;
                }

                // Check for alarm triggers
                self.check_alarms();
            }

            Message::StartStopwatch => {
                self.stopwatch_running = true;
            }

            Message::StopStopwatch => {
                self.stopwatch_running = false;
                // Send notification with current time
                let time_str = format!("{:02}:{:02}:{:02}", 
                    self.stopwatch_time.as_secs() / 3600,
                    (self.stopwatch_time.as_secs() % 3600) / 60,
                    self.stopwatch_time.as_secs() % 60
                );
                notifications::send_stopwatch_notification(&time_str);
            }

            Message::ResetStopwatch => {
                self.stopwatch_running = false;
                self.stopwatch_time = Duration::default();
            }

            Message::StartTimer => {
                self.timer_running = true;
            }

            Message::StopTimer => {
                self.timer_running = false;
            }

            Message::ResetTimer => {
                self.timer_running = false;
                self.timer_remaining = self.timer_duration;
            }

            Message::AddAlarm => {
                self.editing_alarm = Some(AlarmEdit {
                    id: None,
                    hour: self.current_time.hour(),
                    minute: self.current_time.minute(),
                    label: String::new(),
                    repeat_days: [false; 7],
                    snooze_minutes: 5,
                });
            }

            Message::EditAlarm(id) => {
                if let Some(alarm) = self.alarms.iter().find(|a| a.id == id) {
                    self.editing_alarm = Some(AlarmEdit {
                        id: Some(id),
                        hour: alarm.time.hour(),
                        minute: alarm.time.minute(),
                        label: alarm.label.clone(),
                        repeat_days: [false; 7],
                        snooze_minutes: 5,
                    });
                }
            }

            Message::DeleteAlarm(id) => {
                self.alarms.retain(|alarm| alarm.id != id);
            }

            Message::ToggleAlarm(id) => {
                if let Some(alarm) = self.alarms.iter_mut().find(|a| a.id == id) {
                    alarm.enabled = !alarm.enabled;
                }
            }

            Message::SaveAlarm => {
                if let Some(edit) = &self.editing_alarm {
                    let time = chrono::NaiveTime::from_hms_opt(edit.hour, edit.minute, 0)
                        .unwrap_or_else(|| chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap());
                    
                    if let Some(id) = edit.id {
                        // Edit existing alarm
                        if let Some(alarm) = self.alarms.iter_mut().find(|a| a.id == id) {
                            alarm.time = time;
                            alarm.label = edit.label.clone();
                        }
                    } else {
                        // Add new alarm
                        self.alarms.push(AlarmItem {
                            id: self.next_alarm_id,
                            time,
                            label: edit.label.clone(),
                            enabled: true,
                            repeat_days: edit.repeat_days,
                            snooze_minutes: edit.snooze_minutes,
                        });
                        self.next_alarm_id += 1;
                        
                        // Send confirmation notification
                        let _ = notify_rust::Notification::new()
                            .summary("Alarm Set")
                            .body(&format!("⏰ Alarm set for {}", time.format("%H:%M")))
                            .icon("alarm-symbolic")
                            .timeout(notify_rust::Timeout::Milliseconds(2000))
                            .show();
                    }
                    
                    self.editing_alarm = None;
                }
            }

            Message::CancelAlarmEdit => {
                self.editing_alarm = None;
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

            Message::AlarmEditSnoozeMinutes(minutes) => {
                if let Some(edit) = &mut self.editing_alarm {
                    edit.snooze_minutes = minutes.min(60);
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
            Message::SetTimerHours(hours) => {
                self.timer_edit_hours = hours.min(23);
            }

            Message::SetTimerMinutes(minutes) => {
                // This is for timer editing mode
                if self.editing_timer {
                    self.timer_edit_minutes = minutes.min(59);
                } else {
                    self.timer_duration = Duration::from_secs(minutes as u64 * 60 + self.timer_duration.as_secs() % 60);
                    self.timer_remaining = self.timer_duration;
                }
            }

            Message::SetTimerSeconds(seconds) => {
                // This is for timer editing mode
                if self.editing_timer {
                    self.timer_edit_seconds = seconds.min(59);
                } else {
                    self.timer_duration = Duration::from_secs((self.timer_duration.as_secs() / 60) * 60 + seconds as u64);
                    self.timer_remaining = self.timer_duration;
                }
            }

            Message::StartTimerEdit => {
                self.editing_timer = true;
                let total_secs = self.timer_duration.as_secs();
                self.timer_edit_hours = (total_secs / 3600) as u32;
                self.timer_edit_minutes = ((total_secs % 3600) / 60) as u32;
                self.timer_edit_seconds = (total_secs % 60) as u32;
            }

            Message::SaveTimerEdit => {
                self.editing_timer = false;
                self.timer_duration = Duration::from_secs(
                    self.timer_edit_hours as u64 * 3600 +
                    self.timer_edit_minutes as u64 * 60 +
                    self.timer_edit_seconds as u64
                );
                if !self.timer_running {
                    self.timer_remaining = self.timer_duration;
                }
            }

            Message::CancelTimerEdit => {
                self.editing_timer = false;
            }

            // World clock messages
            Message::AddWorldClock => {
                // Add a new world clock with default values
                let offset = self.world_clocks.len() as i32;
                self.world_clocks.push(WorldClockItem {
                    name: format!("City {}", self.world_clocks.len()),
                    timezone: format!("UTC+{}", offset),
                    offset_hours: offset,
                });
            }

            Message::DeleteWorldClock(index) => {
                if index < self.world_clocks.len() && self.world_clocks.len() > 1 {
                    self.world_clocks.remove(index);
                }
            }

            // Stopwatch lap messages
            Message::RecordLap => {
                if self.stopwatch_running {
                    self.lap_times.push(self.stopwatch_time);
                }
            }

            Message::ClearLaps => {
                self.lap_times.clear();
            }
        }
        Task::none()
    }

    /// Called when a nav item is selected.
    fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<cosmic::Action<Self::Message>> {
        self.nav.activate(id);
        self.update_title()
    }
}

impl AppModel {
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

    /// World Clock view
    fn world_clock_view(&self) -> Element<'_, Message> {
        let cosmic_theme::Spacing { space_m, space_l, .. } = theme::active().cosmic().spacing;
        
        let mut column = widget::column()
            .push(widget::text::title1("🌍").size(48.0 * self.pulse_animation))
            .push(widget::text::title1(self.current_time.format("%H:%M:%S").to_string())
                .size(64.0 * self.pulse_animation)
                .align_x(Alignment::Center))
            .push(widget::text::body(self.current_time.format("%A, %B %d, %Y").to_string()).align_x(Alignment::Center))
            .spacing(space_m)
            .align_x(Alignment::Center);

        // Add world clocks
        if self.world_clocks.len() > 1 {
            column = column.push(widget::divider::horizontal::default());
            column = column.push(widget::text::title2("World Clocks").align_x(Alignment::Center));
            
            for (_i, clock) in self.world_clocks.iter().enumerate() {
                let time_at_offset = self.current_time.with_timezone(
                    &chrono::FixedOffset::east_opt(clock.offset_hours * 3600).unwrap_or(chrono::FixedOffset::east_opt(0).unwrap())
                );
                
                let clock_row = widget::row()
                    .push(widget::text::body(&clock.name).width(Length::FillPortion(1)))
                    .push(widget::text::body(time_at_offset.format("%H:%M:%S").to_string()).width(Length::FillPortion(2)))
                    .push(widget::text::body(&clock.timezone).width(Length::FillPortion(1)))
                    .spacing(space_m)
                    .align_y(Vertical::Center);
                
                column = column.push(clock_row);
            }
        }

        column = column.push(
            widget::button::standard("Add City")
                .on_press(Message::AddWorldClock)
                .width(Length::Shrink)
        );

        column
            .apply(widget::container)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .padding(space_l)
            .into()
    }

    /// Alarm view
    fn alarm_view(&self) -> Element<'_, Message> {
        let cosmic_theme::Spacing { space_m, space_l, .. } = theme::active().cosmic().spacing;
        
        if let Some(edit) = &self.editing_alarm {
            // Show alarm edit form
            self.alarm_edit_view(edit)
        } else {
            // Show alarm list
            let mut column = widget::column()
                .push(widget::text::title1("⏰"))
                .push(widget::text::title2(fl!("alarms")))
                .push(widget::button::standard(fl!("add-alarm")).on_press(Message::AddAlarm))
                .spacing(space_m);

            if self.alarms.is_empty() {
                column = column.push(widget::text::body(fl!("no-alarms")));
            } else {
                for alarm in &self.alarms {
                    let alarm_row = widget::row()
                        .push(widget::text::body(alarm.time.format("%H:%M").to_string()))
                        .push(widget::text::body(&alarm.label))
                        .push(
                            widget::toggler(alarm.enabled)
                                .on_toggle(move |_| Message::ToggleAlarm(alarm.id))
                        )
                        .push(widget::button::standard(fl!("edit-alarm")).on_press(Message::EditAlarm(alarm.id)))
                        .push(widget::button::destructive(fl!("delete-alarm")).on_press(Message::DeleteAlarm(alarm.id)))
                        .spacing(space_m)
                        .align_y(Vertical::Center);
                    
                    column = column.push(alarm_row);
                }
            }

            column
                .align_x(Alignment::Center)
                .apply(widget::container)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center)
                .padding(space_l)
                .into()
        }
    }

    /// Alarm edit view
    fn alarm_edit_view(&self, edit: &AlarmEdit) -> Element<'_, Message> {
        let cosmic_theme::Spacing { space_m, space_l, .. } = theme::active().cosmic().spacing;
        
        let hour_str = edit.hour.to_string();
        let minute_str = edit.minute.to_string();

        widget::column()
            .push(widget::text::title2(fl!("add-alarm")))
            .push(
                widget::row()
                    .push(widget::text::body(fl!("hour")))
                    .push(
                        widget::text_input("", hour_str)
                            .on_input(|s| Message::AlarmEditHour(s.parse().unwrap_or(0)))
                    )
                    .push(widget::text::body(fl!("minute")))
                    .push(
                        widget::text_input("", minute_str)
                            .on_input(|s| Message::AlarmEditMinute(s.parse().unwrap_or(0)))
                    )
                    .spacing(space_m)
                    .align_y(Vertical::Center)
            )
            .push(
                widget::text_input(fl!("alarm-label"), edit.label.clone())
                    .on_input(Message::AlarmEditLabel)
            )
            .push(
                widget::row()
                    .push(widget::button::standard(fl!("save-alarm")).on_press(Message::SaveAlarm))
                    .push(widget::button::standard(fl!("reset")).on_press(Message::CancelAlarmEdit))
                    .spacing(space_m)
            )
            .spacing(space_m)
            .align_x(Alignment::Center)
            .apply(widget::container)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .padding(space_l)
            .into()
    }
    fn stopwatch_view(&self) -> Element<'_, Message> {
        let cosmic_theme::Spacing { space_m, space_l, .. } = theme::active().cosmic().spacing;
        
        let time_str = format!("{:02}:{:02}:{:02}", 
            self.stopwatch_time.as_secs() / 3600,
            (self.stopwatch_time.as_secs() % 3600) / 60,
            self.stopwatch_time.as_secs() % 60
        );
        
        let mut column = widget::column()
            .push(widget::text::title1("⏱️").size(48.0 * self.pulse_animation))
            .push(widget::text::title1(time_str.clone())
                .size(72.0 * self.pulse_animation)
                .align_x(Alignment::Center))
            .push(
                widget::row()
                    .push(
                        widget::button::standard(fl!("start"))
                            .on_press(Message::StartStopwatch)
                    )
                    .push(
                        widget::button::standard(fl!("stop"))
                            .on_press(Message::StopStopwatch)
                    )
                    .push(
                        widget::button::standard(fl!("reset"))
                            .on_press(Message::ResetStopwatch)
                    )
                    .push(
                        widget::button::standard("Lap")
                            .on_press(Message::RecordLap)
                    )
                    .spacing(space_m)
            )
            .spacing(space_m)
            .align_x(Alignment::Center);

        // Show lap times
        if !self.lap_times.is_empty() {
            column = column.push(widget::divider::horizontal::default());
            column = column.push(widget::text::title2("Laps").align_x(Alignment::Center));
            column = column.push(
                widget::button::standard("Clear Laps")
                    .on_press(Message::ClearLaps)
                    .width(Length::Shrink)
            );
            
            // Show last 10 laps in reverse order (most recent first)
            let laps_to_show = self.lap_times.iter().rev().take(10);
            for (idx, lap) in laps_to_show.enumerate() {
                let lap_str = format!("{:02}:{:02}:{:02}.{:03}", 
                    lap.as_secs() / 3600,
                    (lap.as_secs() % 3600) / 60,
                    lap.as_secs() % 60,
                    (lap.as_millis() % 1000) as u32 / 10
                );
                let lap_num = self.lap_times.len() - idx;
                column = column.push(widget::text::body(format!("Lap {}: {}", lap_num, lap_str)));
            }
        }

        column
            .apply(widget::container)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .padding(space_l)
            .into()
    }

    /// Timer view
    fn timer_view(&self) -> Element<'_, Message> {
        let cosmic_theme::Spacing { space_m, space_l, .. } = theme::active().cosmic().spacing;
        
        if self.editing_timer {
            // Timer edit view
            let mut column = widget::column()
                .push(widget::text::title1("⏲️ Set Timer").size(36.0))
                .push(
                    widget::row()
                        .push(
                            widget::column()
                                .push(widget::text::body("Hours"))
                                .push(
                                    widget::text_input("", self.timer_edit_hours.to_string())
                                        .on_input(|s| Message::SetTimerHours(s.parse().unwrap_or(0)))
                                        .width(Length::Fixed(80.0))
                                )
                                .spacing(space_m)
                                .align_x(Alignment::Center)
                        )
                        .push(
                            widget::column()
                                .push(widget::text::body("Minutes"))
                                .push(
                                    widget::text_input("", self.timer_edit_minutes.to_string())
                                        .on_input(|s| Message::SetTimerMinutes(s.parse().unwrap_or(0)))
                                        .width(Length::Fixed(80.0))
                                )
                                .spacing(space_m)
                                .align_x(Alignment::Center)
                        )
                        .push(
                            widget::column()
                                .push(widget::text::body("Seconds"))
                                .push(
                                    widget::text_input("", self.timer_edit_seconds.to_string())
                                        .on_input(|s| Message::SetTimerSeconds(s.parse().unwrap_or(0)))
                                        .width(Length::Fixed(80.0))
                                )
                                .spacing(space_m)
                                .align_x(Alignment::Center)
                        )
                        .spacing(space_l)
                )
                .push(
                    widget::row()
                        .push(widget::button::standard("Save").on_press(Message::SaveTimerEdit))
                        .push(widget::button::standard("Cancel").on_press(Message::CancelTimerEdit))
                        .spacing(space_m)
                )
                .spacing(space_l)
                .align_x(Alignment::Center);

            // Timer presets
            column = column.push(widget::divider::horizontal::default());
            column = column.push(widget::text::body("Quick Presets").align_x(Alignment::Center));
            column = column.push(
                widget::row()
                    .push(widget::button::standard("1 min").on_press(Message::SetTimerMinutes(1)))
                    .push(widget::button::standard("5 min").on_press(Message::SetTimerMinutes(5)))
                    .push(widget::button::standard("10 min").on_press(Message::SetTimerMinutes(10)))
                    .push(widget::button::standard("30 min").on_press(Message::SetTimerMinutes(30)))
                    .push(widget::button::standard("1 hour").on_press(Message::SetTimerMinutes(60)))
                    .spacing(space_m)
            );

            column
                .apply(widget::container)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center)
                .padding(space_l)
                .into()
        } else {
            let time_str = format!("{:02}:{:02}", 
                self.timer_remaining.as_secs() / 60,
                self.timer_remaining.as_secs() % 60
            );
            
            let is_low = self.timer_remaining.as_secs() <= 10 && self.timer_running;
            let timer_scale = if is_low { self.pulse_animation } else { 1.0 };
            
            widget::column()
                .push(widget::text::title1("⏲️").size(48.0 * self.pulse_animation))
                .push(widget::text::title1(time_str)
                    .size(72.0 * timer_scale)
                    .align_x(Alignment::Center))
                .push(
                    widget::row()
                        .push(
                            widget::button::standard(fl!("start"))
                                .on_press(Message::StartTimer)
                        )
                        .push(
                            widget::button::standard(fl!("stop"))
                                .on_press(Message::StopTimer)
                        )
                        .push(
                            widget::button::standard(fl!("reset"))
                                .on_press(Message::ResetTimer)
                        )
                        .push(
                            widget::button::standard("Edit")
                                .on_press(Message::StartTimerEdit)
                        )
                        .spacing(space_m)
                )
                .spacing(space_m)
                .align_x(Alignment::Center)
                .apply(widget::container)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center)
                .padding(space_l)
                .into()
        }
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
#[derive(Clone, Debug, Default)]
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

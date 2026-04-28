// SPDX-License-Identifier: MPL-2.0

use crate::config::Config;
use crate::fl;
use cosmic::app::context_drawer;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, Length, Subscription, time};
use cosmic::prelude::*;
use cosmic::widget::{self, icon, menu, nav_bar, button, container, column, row, text};
use cosmic::{cosmic_theme, theme};
use futures_util::SinkExt;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

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
    // Configuration data that persists between application runs.
    config: Config,
    /// Current time for clock display
    current_time: String,
    /// Timer duration in seconds
    timer_seconds: u64,
    /// Timer is running
    timer_running: bool,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    OpenRepositoryUrl,
    SubscriptionChannel,
    ToggleContextPage(ContextPage),
    UpdateConfig(Config),
    LaunchUrl(String),
    UpdateTime,
    StartTimer,
    PauseTimer,
    ResetTimer,
    IncrementTimer,
    DecrementTimer,
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
        // Create a nav bar with three page items.
        let mut nav = nav_bar::Model::default();

        nav.insert()
            .text(fl!("page-id", num = 1))
            .data::<Page>(Page::Clock)
            .icon(icon::from_name("appointment-soon-symbolic"))
            .activate();

        nav.insert()
            .text(fl!("page-id", num = 2))
            .data::<Page>(Page::Timer)
            .icon(icon::from_name("chronometer-symbolic"));

        nav.insert()
            .text(fl!("page-id", num = 3))
            .data::<Page>(Page::Alarms)
            .icon(icon::from_name("alarm-symbolic"));

        // Construct the app model with the runtime's core.
        let mut app = AppModel {
            core,
            context_page: ContextPage::default(),
            nav,
            key_binds: HashMap::new(),
            // Optional configuration file for an application.
            config: cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
                .map(|context| match Config::get_entry(&context) {
                    Ok(config) => config,
                    Err((_errors, config)) => {
                        // for why in errors {
                        //     tracing::error!(%why, "error loading app config");
                        // }

                        config
                    }
                })
                .unwrap_or_default(),
            current_time: String::from("00:00:00"),
            timer_seconds: 300,
            timer_running: false,
        };

        // Create a startup command that sets the window title.
        let command = app.update_title();

        (app, command)
    }

    /// Elements to pack at the start of the header bar.
    fn header_start(&self) -> Vec<Element<Self::Message>> {
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

    /// Display a context drawer if the context page is requested.
    fn context_drawer(&self) -> Option<context_drawer::ContextDrawer<Self::Message>> {
        if !self.core.window.show_context {
            return None;
        }

        Some(match self.context_page {
            ContextPage::About => context_drawer::context_drawer(
                self.about(),
                Message::ToggleContextPage(ContextPage::About),
            )
            .title(fl!("about")),
        })
    }

    /// Describes the interface based on the current state of the application model.
    ///
    /// Application events will be processed through the view. Any messages emitted by
    /// events received by widgets will be passed to the update method.
    fn view(&self) -> Element<Self::Message> {
        let spacing = &theme::active().cosmic().spacing;
        let active_page = self.nav.active_data::<Page>();

        let content = match active_page {
            Some(Page::Clock) => self.clock_page(),
            Some(Page::Timer) => self.timer_page(),
            Some(Page::Alarms) => self.alarms_page(),
            None => Element::from(text("No page selected")),
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(spacing.space_m)
            .into()
    }

    /// Register subscriptions for this application.
    ///
    /// Subscriptions are long-running async tasks running in the background which
    /// emit messages to the application through a channel. They are started at the
    /// beginning of the application, and persist through its lifetime.
    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::batch(vec![
            // Update time every 500ms
            time::every(std::time::Duration::from_millis(500))
                .map(|_| Message::UpdateTime),
            // Watch for application configuration changes.
            self.core()
                .watch_config::<Config>(Self::APP_ID)
                .map(|update| {
                    // for why in update.errors {
                    //     tracing::error!(?why, "app config error");
                    // }

                    Message::UpdateConfig(update.config)
                }),
        ])
    }

    /// Handles messages emitted by the application and its widgets.
    ///
    /// Tasks may be returned for asynchronous execution of code in the background
    /// on the application's async runtime.
    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::OpenRepositoryUrl => {
                _ = open::that_detached(REPOSITORY);
            }

            Message::SubscriptionChannel => {
                // For example purposes only.
            }

            Message::ToggleContextPage(context_page) => {
                if self.context_page == context_page {
                    // Close the context drawer if the toggled context page is the same.
                    self.core.window.show_context = !self.core.window.show_context;
                } else {
                    // Open the context drawer to display the requested context page.
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
                let now = std::time::SystemTime::now();
                let duration = now
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default();
                
                let secs = duration.as_secs();
                let h = (secs / 3600) % 24;
                let m = (secs / 60) % 60;
                let s = secs % 60;
                
                self.current_time = format!("{:02}:{:02}:{:02}", h, m, s);

                if self.timer_running && self.timer_seconds > 0 {
                    self.timer_seconds -= 1;
                }
            }

            Message::StartTimer => {
                self.timer_running = true;
            }

            Message::PauseTimer => {
                self.timer_running = false;
            }

            Message::ResetTimer => {
                self.timer_running = false;
                self.timer_seconds = 300;
            }

            Message::IncrementTimer => {
                if !self.timer_running {
                    self.timer_seconds += 60;
                }
            }

            Message::DecrementTimer => {
                if !self.timer_running && self.timer_seconds > 0 {
                    self.timer_seconds = self.timer_seconds.saturating_sub(60);
                }
            }
        }
        Task::none()
    }

    /// Called when a nav item is selected.
    fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<cosmic::Action<Self::Message>> {
        // Activate the page in the model.
        self.nav.activate(id);

        self.update_title()
    }
}

impl AppModel {
    /// The about page for this app.
    pub fn about(&self) -> Element<Message> {
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

    /// Clock page showing current time
    pub fn clock_page(&self) -> Element<Message> {
        let spacing = &theme::active().cosmic().spacing;
        
        column()
            .push(
                container(
                    text::title1(&self.current_time)
                        .width(Length::Fill)
                        .height(Length::Fill)
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center)
                .padding(spacing.space_l)
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .spacing(spacing.space_m)
            .into()
    }

    /// Timer page
    pub fn timer_page(&self) -> Element<Message> {
        let spacing = &theme::active().cosmic().spacing;
        
        let minutes = self.timer_seconds / 60;
        let seconds = self.timer_seconds % 60;
        let timer_display = format!("{:02}:{:02}", minutes, seconds);

        let button_row = row()
            .push(
                button(text("−").size(20))
                    .on_press(Message::DecrementTimer)
                    .width(Length::Fixed(60.0))
                    .height(Length::Fixed(60.0))
                    .padding(10)
            )
            .push(
                if self.timer_running {
                    button(text("Pause").horizontal_alignment(Horizontal::Center))
                        .on_press(Message::PauseTimer)
                        .width(Length::Fixed(100.0))
                        .height(Length::Fixed(60.0))
                        .padding(10)
                } else {
                    button(text("Start").horizontal_alignment(Horizontal::Center))
                        .on_press(Message::StartTimer)
                        .width(Length::Fixed(100.0))
                        .height(Length::Fixed(60.0))
                        .padding(10)
                }
            )
            .push(
                button(text("+").size(20))
                    .on_press(Message::IncrementTimer)
                    .width(Length::Fixed(60.0))
                    .height(Length::Fixed(60.0))
                    .padding(10)
            )
            .push(
                button(text("Reset").horizontal_alignment(Horizontal::Center))
                    .on_press(Message::ResetTimer)
                    .width(Length::Fixed(80.0))
                    .height(Length::Fixed(60.0))
                    .padding(10)
            )
            .spacing(spacing.space_m)
            .align_y(Vertical::Center);

        column()
            .push(
                container(text::title1(&timer_display))
                    .width(Length::Fill)
                    .height(Length::Fixed(200.0))
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center)
                    .padding(spacing.space_l)
            )
            .push(
                container(button_row)
                    .width(Length::Fill)
                    .align_x(Horizontal::Center)
                    .padding(spacing.space_m)
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .spacing(spacing.space_m)
            .align_x(Alignment::Center)
            .into()
    }

    /// Alarms page
    pub fn alarms_page(&self) -> Element<Message> {
        let spacing = &theme::active().cosmic().spacing;
        
        column()
            .push(text::title2("Alarms"))
            .push(
                container(
                    column()
                        .push(
                            button(text("+ Add Alarm").horizontal_alignment(Horizontal::Center))
                                .width(Length::Fixed(150.0))
                                .height(Length::Fixed(50.0))
                                .padding(10)
                        )
                        .push(text("No alarms set"))
                        .spacing(spacing.space_m)
                )
                .padding(spacing.space_m)
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .spacing(spacing.space_m)
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
pub enum Page {
    Clock,
    Timer,
    Alarms,
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

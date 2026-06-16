# COSMIC Watch

A clock, alarm, stopwatch, and timer app for the COSMIC desktop — inspired by the macOS Clock app.

## Features

- 🌍 **World Clock** — Analog clock faces per city with day/night fill, date-aware time difference labels ("Today, 3h ahead", "Tomorrow"), delete-on-hover, ←↑↓→ reorder, West-to-East UTC sort, searchable city picker
- ⏰ **Alarms** — macOS-style cards, 24h time, repeat days with "Every Day" quick toggle + individual day buttons, sound dropdown picker, snooze checkbox + duration dropdown, full edit overlay
- ⏱️ **Stopwatch** — macOS-style mm:ss.cc display, Lap/Split/Total table (most recent first), Start(green)/Stop(red)/Lap(gray)/Reset(gray) buttons
- ⏲️ **Timer** — Multiple concurrent timers, segmented HH:MM:SS editable input with orange active segment, hr/min/sec labels, timer name, Cancel + Start, Pause/Resume/Cancel, "+" to add timers
- 🔔 **System notifications** for alarms, timers, and stopwatch
- 🌙 **Dark mode** — Follows COSMIC system preference automatically
- 🌐 **Localized** via FTL strings

## Dependencies

```bash
# Ubuntu/Debian
sudo apt install libasound2-dev pkg-config build-essential
```

## Build & Install

```bash
git clone https://github.com/Moon-Mind/cosmic-watch.git
cd cosmic-watch
sudo just install


## Project Structure

```
src/
├── main.rs           # Entry point
├── app.rs            # UI and application logic
├── config.rs         # Persistent config
├── notifications.rs  # System notifications
└── i18n.rs           # Localization
i18n/
└── en/
    └── cosmic_watch.ftl  # UI strings
resources/
├── app.desktop
├── app.metainfo.xml
└── icons/
    └── hicolor/scalable/apps/
```

## License

MPL-2.0

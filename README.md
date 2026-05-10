# COSMIC Watch

A clock application for the COSMIC desktop with world clock, alarms, stopwatch, and timer.

## Features

- 🌍 **World Clock** - Current time, date, and multi-city support
- ⏰ **Alarms** - Multiple alarms with labels, repeat days, and snooze
- ⏱️ **Stopwatch** - Start/stop/pause with lap recording
- ⏲️ **Timer** - Countdown with visual progress and presets
- 🔔 **Notifications** - System notifications for alarms, timer, and stopwatch

## Dependencies

```bash
# Ubuntu/Debian
sudo apt install libasound2-dev pkg-config build-essential
```

## Build & Install

```bash
git clone https://github.com/Moon-Mind/cosmic-watch.git
cd cosmic-watch
just build-release   # or: cargo build --release
just run             # or: cargo run

# Install (system-wide)
sudo just install

# Install (user-local)
install -Dm755 target/release/cosmic-watch ~/.local/bin/cosmic-watch
install -Dm644 resources/icons/hicolor/scalable/apps/icon.svg ~/.local/share/icons/hicolor/scalable/apps/cosmic-watch.svg
cp cosmic-watch.desktop ~/.local/share/applications/
```

## Usage

- **World Clock**: Shows local time. Add cities to view multiple timezones.
- **Alarms**: Add, edit, toggle, and delete alarms. Set labels and repeat days.
- **Stopwatch**: Start/stop/pause timing with lap recording.
- **Timer**: Set duration via presets or custom input. Visual countdown circle.
- **Keyboard shortcuts**: `Ctrl+1`-`Ctrl+4` to switch between tabs.

## Commands

| Command | Description |
|---------|-------------|
| `just build-release` | Release build |
| `just build-debug` | Debug build |
| `just run` | Run in debug mode |
| `just check` | Run clippy |
| `just clean` | Clean artifacts |

## Project Structure

```
src/
├── main.rs          # Entry point
├── app.rs           # UI and application logic
├── config.rs        # Persistent config
├── notifications.rs # System notifications
└── i18n.rs          # Localization
```

## License

MPL-2.0
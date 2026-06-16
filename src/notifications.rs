use notify_rust::{Notification, Timeout};
use std::io::Write;
use std::time::Duration;

pub fn send_alarm_notification(label: &str, time: &str) {
    play_rodio_sound("alarm");
    
    let _ = Notification::new()
        .summary("🔔 Alarm")
        .body(&format!("⏰ {}\nTime: {}", label, time))
        .icon("alarm-symbolic")
        .timeout(Timeout::Milliseconds(10000))
        .urgency(notify_rust::Urgency::Critical)
        .show();
}

pub fn send_stopwatch_notification(time: &str) {
    play_rodio_sound("message");

    let _ = Notification::new()
        .summary("⏱️ Stopwatch Stopped")
        .body(&format!("Final time: {}", time))
        .icon("alarm-symbolic")
        .timeout(Timeout::Milliseconds(3000))
        .urgency(notify_rust::Urgency::Normal)
        .show();
}

pub fn send_timer_notification() {
    play_rodio_sound("complete");
    
    let _ = Notification::new()
        .summary("🔔 Timer Finished")
        .body("⏲️ Your timer has finished!")
        .icon("timer-symbolic")
        .timeout(Timeout::Milliseconds(8000))
        .urgency(notify_rust::Urgency::Critical)
        .show();
}

#[allow(dead_code)]
pub fn send_alarm_set_notification(time: &str) {
    let _ = Notification::new()
        .summary("✅ Alarm Set")
        .body(&format!("Alarm scheduled for {}", time))
        .icon("alarm-symbolic")
        .timeout(Timeout::Milliseconds(2000))
        .urgency(notify_rust::Urgency::Low)
        .show();
}

fn play_rodio_sound(sound_type: &str) {
    let sound_type = sound_type.to_string();
    std::thread::spawn(move || {
        // Try rodio first with generated tones
        if let Ok((stream, stream_handle)) = rodio::OutputStream::try_default() {
            let sink = rodio::Sink::try_new(&stream_handle);
            if let Ok(sink) = sink {
                // Generate simple beep tone as WAV data
                let sample_rate = 44100;
                let duration_secs = match sound_type.as_str() {
                    "alarm" => 1.5,
                    "complete" => 0.8,
                    _ => 0.3,
                };
                let num_samples = (sample_rate as f32 * duration_secs) as usize;
                let frequency = match sound_type.as_str() {
                    "alarm" => 880.0,
                    "complete" => 660.0,
                    _ => 440.0,
                };
                
                let samples: Vec<i16> = (0..num_samples).map(|i| {
                    let t = i as f32 / sample_rate as f32;
                    let envelope = if sound_type.as_str() == "alarm" {
                        // Pulsing alarm pattern
                        if (t * 4.0).sin() > 0.0 { 1.0 } else { 0.2 }
                    } else {
                        (1.0 - t / duration_secs).max(0.0)
                    };
                    (t * frequency * std::f32::consts::PI * 2.0).sin() as i16 * (30000.0 * envelope) as i16
                }).collect();
                
                let source = rodio::buffer::SamplesBuffer::new(
                    1,
                    sample_rate,
                    samples,
                );
                sink.append(source);
                sink.sleep_until_end();
                drop(stream);
                return;
            }
        }
        
        // Fallback: try system sound commands
        let sound_name = match sound_type.as_str() {
            "alarm" => "alarm-clock-elapsed",
        "complete" => "complete",
            "message" => "message-new-instant",
            _ => "bell",
        };
        
        let methods = vec![
            ("canberra-gtk-play", vec!["-i", sound_name]),
            ("paplay", vec!["/usr/share/sounds/freedesktop/stereo/bell.oga"]),
            ("aplay", vec!["/usr/share/sounds/alsa/Front_Left.wav"]),
        ];
        
        for (cmd, args) in methods {
            if std::process::Command::new(cmd).args(&args).output().is_ok() {
                return;
            }
        }
        
        // Last resort: terminal beep
        let repeat_count = match sound_type.as_str() {
            "alarm" => 4,
            "complete" => 2,
            _ => 1,
        };
        
        for i in 0..repeat_count {
            print!("\x07");
            std::io::stdout().flush().ok();
            if i < repeat_count - 1 {
                std::thread::sleep(Duration::from_millis(200));
            }
        }
    });
}

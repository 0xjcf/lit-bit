#![cfg_attr(target_arch = "riscv32", no_std)]
#![cfg_attr(target_arch = "riscv32", no_main)]

//! # Parallel States Media Player Example
//!
//! This example demonstrates the use of **parallel states** in a media player.
//! The media player has three orthogonal (independent) regions that operate concurrently:
//!
//! 1. **`PlaybackControl`**: Manages play/pause/stop functionality
//! 2. **`AudioSettings`**: Manages volume and mute state
//! 3. **`DisplayState`**: Manages screen brightness and power
//!
//! These regions are independent - you can be Playing + Muted + `ScreenOff` simultaneously,
//! or any other combination. This showcases how parallel states enable modeling
//! complex systems with multiple concurrent concerns.

#[cfg(target_arch = "riscv32")]
use panic_halt as _;

use lit_bit_core::StateMachine;
use lit_bit_macro::statechart;
use lit_bit_macro::statechart_event;

use core::convert::TryFrom;
use heapless::String;

// Capacities for heapless collections
const TRACK_ID_CAPACITY: usize = 64;
const LOG_CAPACITY: usize = 32;
const LOG_STRING_CAPACITY: usize = 64;

#[derive(Debug, Clone, Default)]
pub struct MediaPlayerContext {
    pub current_track: Option<String<TRACK_ID_CAPACITY>>,
    pub volume: u8,     // 0-100
    pub brightness: u8, // 0-100
    pub action_log: heapless::Vec<String<LOG_STRING_CAPACITY>, LOG_CAPACITY>,
}

impl MediaPlayerContext {
    fn log_action(&mut self, action: &str) {
        if let Ok(log_entry) = String::try_from(action) {
            let _ = self.action_log.push(log_entry);
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[statechart_event]
pub enum MediaPlayerEvent {
    #[default]
    // Playback Control Events
    Play,
    Pause,
    Stop,
    LoadTrack {
        path: String<TRACK_ID_CAPACITY>,
    },

    // Audio Control Events
    VolumeUp,
    VolumeDown,
    ToggleMute,

    // Display Control Events
    ScreenToggle,
    BrightnessUp,
    BrightnessDown,

    // Global Events (affect multiple regions)
    PowerOff,
    PowerOn,
}

// =============================================================================
// PLAYBACK CONTROL REGION ACTIONS & GUARDS
// =============================================================================

fn is_track_loaded(context: &MediaPlayerContext, _event: &MediaPlayerEvent) -> bool {
    context.current_track.is_some()
}

fn guard_valid_track_path(_context: &MediaPlayerContext, event: &MediaPlayerEvent) -> bool {
    if let MediaPlayerEvent::LoadTrack { path } = event {
        !path.is_empty() && path.contains('.')
    } else {
        false
    }
}

fn action_load_track(context: &mut MediaPlayerContext, event: &MediaPlayerEvent) {
    if let MediaPlayerEvent::LoadTrack { path } = event {
        context.current_track = Some(path.clone());
        context.log_action("LoadedTrack");
        #[cfg(not(target_arch = "riscv32"))]
        println!("🎵 Loaded track: {path}");
    }
}

fn action_start_playback(context: &mut MediaPlayerContext, _event: &MediaPlayerEvent) {
    context.log_action("StartedPlayback");
    #[cfg(not(target_arch = "riscv32"))]
    if let Some(track) = &context.current_track {
        println!("▶️  Playing: {track}");
    }
}

fn action_pause_playback(context: &mut MediaPlayerContext, _event: &MediaPlayerEvent) {
    context.log_action("PausedPlayback");
    #[cfg(not(target_arch = "riscv32"))]
    println!("⏸️  Playback paused");
}

fn action_stop_playback(context: &mut MediaPlayerContext, _event: &MediaPlayerEvent) {
    context.log_action("StoppedPlayback");
    #[cfg(not(target_arch = "riscv32"))]
    println!("⏹️  Playback stopped");
}

// =============================================================================
// AUDIO SETTINGS REGION ACTIONS
// =============================================================================

fn action_volume_up(context: &mut MediaPlayerContext, _event: &MediaPlayerEvent) {
    if context.volume < 100 {
        context.volume = (context.volume + 10).min(100);
        context.log_action("VolumeUp");
        #[cfg(not(target_arch = "riscv32"))]
        println!("🔊 Volume: {}", context.volume);
    }
}

fn action_volume_down(context: &mut MediaPlayerContext, _event: &MediaPlayerEvent) {
    if context.volume > 0 {
        context.volume = context.volume.saturating_sub(10);
        context.log_action("VolumeDown");
        #[cfg(not(target_arch = "riscv32"))]
        println!("🔉 Volume: {}", context.volume);
    }
}

fn action_mute(context: &mut MediaPlayerContext, _event: &MediaPlayerEvent) {
    context.log_action("Muted");
    #[cfg(not(target_arch = "riscv32"))]
    println!("🔇 Audio muted");
}

fn action_unmute(context: &mut MediaPlayerContext, _event: &MediaPlayerEvent) {
    context.log_action("Unmuted");
    #[cfg(not(target_arch = "riscv32"))]
    println!("🔊 Audio unmuted (Volume: {})", context.volume);
}

// =============================================================================
// DISPLAY STATE REGION ACTIONS
// =============================================================================

fn action_screen_on(context: &mut MediaPlayerContext, _event: &MediaPlayerEvent) {
    context.log_action("ScreenOn");
    #[cfg(not(target_arch = "riscv32"))]
    println!("💡 Screen ON (Brightness: {})", context.brightness);
}

fn action_screen_off(context: &mut MediaPlayerContext, _event: &MediaPlayerEvent) {
    context.log_action("ScreenOff");
    #[cfg(not(target_arch = "riscv32"))]
    println!("🌑 Screen OFF");
}

fn action_brightness_up(context: &mut MediaPlayerContext, _event: &MediaPlayerEvent) {
    if context.brightness < 100 {
        context.brightness = (context.brightness + 20).min(100);
        context.log_action("BrightnessUp");
        #[cfg(not(target_arch = "riscv32"))]
        println!("☀️  Brightness: {}", context.brightness);
    }
}

fn action_brightness_down(context: &mut MediaPlayerContext, _event: &MediaPlayerEvent) {
    if context.brightness > 0 {
        context.brightness = context.brightness.saturating_sub(20);
        context.log_action("BrightnessDown");
        #[cfg(not(target_arch = "riscv32"))]
        println!("🌙 Brightness: {}", context.brightness);
    }
}

// =============================================================================
// GLOBAL ACTIONS (affect multiple regions)
// =============================================================================

fn action_power_off(context: &mut MediaPlayerContext, _event: &MediaPlayerEvent) {
    context.log_action("PowerOff");
    #[cfg(not(target_arch = "riscv32"))]
    println!("⚡ System powering off...");
}

fn action_power_on(context: &mut MediaPlayerContext, _event: &MediaPlayerEvent) {
    context.brightness = 50; // Reset to default
    context.volume = 50; // Reset to default
    context.log_action("PowerOn");
    #[cfg(not(target_arch = "riscv32"))]
    println!("⚡ System powered on - defaults restored");
}

// =============================================================================
// PARALLEL STATES STATECHART DEFINITION
// =============================================================================

statechart! {
    name: ParallelMediaPlayer,
    context: MediaPlayerContext,
    event: MediaPlayerEvent,
    initial: MediaPlayerOperational,

    // The main operational state with three parallel regions
    state MediaPlayerOperational [parallel] {
        // Global transitions that exit all regions
        on MediaPlayerEvent::PowerOff => PoweredOff [action action_power_off];

        // REGION 1: Playback Control
        state PlaybackControl {
            initial: Stopped;

            state Stopped {
                on MediaPlayerEvent::LoadTrack { path: _ } [guard guard_valid_track_path] => Stopped [action action_load_track];
                on MediaPlayerEvent::Play [guard is_track_loaded] => Playing [action action_start_playback];
            }

            state Playing {
                on MediaPlayerEvent::Pause => Paused [action action_pause_playback];
                on MediaPlayerEvent::Stop => Stopped [action action_stop_playback];
            }

            state Paused {
                on MediaPlayerEvent::Play => Playing [action action_start_playback];
                on MediaPlayerEvent::Stop => Stopped [action action_stop_playback];
            }
        }

        // REGION 2: Audio Settings
        state AudioSettings {
            initial: Normal;

            state Normal {
                on MediaPlayerEvent::VolumeUp => Normal [action action_volume_up];
                on MediaPlayerEvent::VolumeDown => Normal [action action_volume_down];
                on MediaPlayerEvent::ToggleMute => Muted [action action_mute];
            }

            state Muted {
                on MediaPlayerEvent::ToggleMute => Normal [action action_unmute];
                // Volume controls are ignored when muted
                on MediaPlayerEvent::VolumeUp => Muted;
                on MediaPlayerEvent::VolumeDown => Muted;
            }
        }

        // REGION 3: Display State
        state DisplayState {
            initial: ScreenOn;

            state ScreenOn {
                on MediaPlayerEvent::ScreenToggle => ScreenOff [action action_screen_off];
                on MediaPlayerEvent::BrightnessUp => ScreenOn [action action_brightness_up];
                on MediaPlayerEvent::BrightnessDown => ScreenOn [action action_brightness_down];
            }

            state ScreenOff {
                on MediaPlayerEvent::ScreenToggle => ScreenOn [action action_screen_on];
                // Brightness controls are ignored when screen is off
                on MediaPlayerEvent::BrightnessUp => ScreenOff;
                on MediaPlayerEvent::BrightnessDown => ScreenOff;
            }
        }
    }

    // Single atomic state - system is completely off
    state PoweredOff {
        on MediaPlayerEvent::PowerOn => MediaPlayerOperational [action action_power_on];
    }
}

#[cfg(not(target_arch = "riscv32"))]
fn main() {
    println!("🎯 Parallel States Media Player Example");
    println!("========================================");

    let initial_context = MediaPlayerContext {
        current_track: None,
        volume: 50,
        brightness: 50,
        action_log: heapless::Vec::new(),
    };

    let mut player = ParallelMediaPlayer::new(initial_context, &MediaPlayerEvent::default())
        .expect("Failed to create parallel media player");

    println!("\n📊 Initial State:");
    println!("Active states: {:?}", player.state());
    println!(
        "Context: Volume={}, Brightness={}",
        player.context().volume,
        player.context().brightness
    );

    // Demonstrate parallel region independence
    println!("\n🎵 Loading and playing a track...");
    player.send(&MediaPlayerEvent::LoadTrack {
        path: String::try_from("awesome_song.mp3").unwrap(),
    });
    player.send(&MediaPlayerEvent::Play);

    println!("\n🔊 Adjusting audio while playing...");
    player.send(&MediaPlayerEvent::VolumeUp);
    player.send(&MediaPlayerEvent::VolumeUp);
    player.send(&MediaPlayerEvent::ToggleMute);

    println!("\n💡 Controlling display independently...");
    player.send(&MediaPlayerEvent::BrightnessDown);
    player.send(&MediaPlayerEvent::ScreenToggle);

    println!("\n📊 Current State (Playing + Muted + ScreenOff):");
    println!("Active states: {:?}", player.state());

    println!("\n🔄 Unmuting and turning screen back on...");
    player.send(&MediaPlayerEvent::ToggleMute);
    player.send(&MediaPlayerEvent::ScreenToggle);

    println!("\n⏸️  Pausing playback (audio/display unaffected)...");
    player.send(&MediaPlayerEvent::Pause);

    println!("\n📊 Final State (Paused + Normal + ScreenOn):");
    println!("Active states: {:?}", player.state());
    println!(
        "Final context: Volume={}, Brightness={}",
        player.context().volume,
        player.context().brightness
    );

    println!("\n📝 Action Log:");
    for (i, action) in player.context().action_log.iter().enumerate() {
        println!("  {}. {}", i + 1, action);
    }

    println!("\n⚡ Testing global power off...");
    player.send(&MediaPlayerEvent::PowerOff);
    println!("Active states after power off: {:?}", player.state());

    println!("\n✅ Parallel states demo complete!");
    println!("This example shows how 3 independent regions can operate concurrently:");
    println!("- PlaybackControl: Stopped/Playing/Paused");
    println!("- AudioSettings: Normal/Muted (+ volume levels)");
    println!("- DisplayState: ScreenOn/ScreenOff (+ brightness levels)");
}

#[cfg(target_arch = "riscv32")]
// Entry point for RISC-V targets - no main function
#[riscv_rt::entry]
fn riscv_main() -> ! {
    loop {
        // Minimal loop for no_std target
    }
}

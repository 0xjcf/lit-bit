#![cfg_attr(target_arch = "riscv32", no_std)]
#![cfg_attr(target_arch = "riscv32", no_main)]

// This example is primarily for host testing for now.
// no_std string handling will be refined later.

#[cfg(target_arch = "riscv32")]
use panic_halt as _;

use lit_bit_core::StateMachine;
use lit_bit_macro::statechart;
use lit_bit_macro::statechart_event;

use core::convert::TryFrom; // ← required for String::try_from

use heapless::String; // Removed Vec

// Capacities for heapless collections
const _LOG_CAPACITY: usize = 64;
const _ACTION_LOG_STRING_CAPACITY: usize = 64;
const TRACK_ID_CAPACITY: usize = 64;
const _STATUS_MSG_CAPACITY: usize = 128;

#[derive(Debug, Clone, Default)]
pub struct MediaPlayerContext {
    // Simplified for initial test
    pub current_track: Option<String<TRACK_ID_CAPACITY>>,
    pub volume: u8,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[statechart_event]
pub enum MediaPlayerEvent {
    #[default]
    Play,
    Stop,
    // Load a new track from the file-system (payload = file path)
    LoadTrack {
        path: String<TRACK_ID_CAPACITY>,
    },
    VolumeUp,
    VolumeDown,
    NextTrack,
    PrevTrack,
    PowerOff,
}

// Action and Guard functions (simplified)
fn do_play(context: &mut MediaPlayerContext, _event: &MediaPlayerEvent) {
    if let Some(track) = &context.current_track {
        #[cfg(not(target_arch = "riscv32"))]
        println!("[Action] do_play - Track: {track:?}");
    }
}

fn do_stop(_context: &mut MediaPlayerContext, _event: &MediaPlayerEvent) {
    #[cfg(not(target_arch = "riscv32"))]
    println!("Stopping playback.");
}

fn is_track_loaded(context: &MediaPlayerContext, _event: &MediaPlayerEvent) -> bool {
    let loaded = context.current_track.is_some();
    #[cfg(not(target_arch = "riscv32"))]
    println!("[Guard] is_track_loaded? {loaded}");
    loaded
}

// -------------- guard & action for LoadTrack --------------
fn guard_for_load_track(_ctx: &MediaPlayerContext, ev: &MediaPlayerEvent) -> bool {
    if let MediaPlayerEvent::LoadTrack { path } = ev {
        // Robust path validation: check for valid file extension when std is available
        #[cfg(not(target_arch = "riscv32"))]
        {
            use std::path::Path;
            let ok = !path.is_empty() && Path::new(path.as_str()).extension().is_some();
            println!("[Guard] guard_for_load_track('{path}')? {ok}");
            ok
        }

        // Fallback validation for no_std environments (RISC-V)
        #[cfg(target_arch = "riscv32")]
        {
            // More careful validation: check for valid file extension pattern
            // Must be non-empty, contain a dot, and have content after the last dot
            let ok = !path.is_empty()
                && path.contains('.')
                && path
                    .rfind('.')
                    .map_or(false, |dot_pos| dot_pos < path.len() - 1 && dot_pos > 0);
            ok
        }
    } else {
        false
    }
}

fn action_for_load_track(ctx: &mut MediaPlayerContext, ev: &MediaPlayerEvent) {
    if let MediaPlayerEvent::LoadTrack { path } = ev {
        #[cfg(not(target_arch = "riscv32"))]
        println!("[Action] action_for_load_track – Path: {path}");
        ctx.current_track = Some(path.clone());
    }
}

fn entry_stopped(ctx: &mut MediaPlayerContext, _event: &MediaPlayerEvent) {
    #[cfg(not(target_arch = "riscv32"))]
    println!("Entering Stopped state. Track: {:?}", ctx.current_track);
}

fn exit_stopped(ctx: &mut MediaPlayerContext, _event: &MediaPlayerEvent) {
    #[cfg(not(target_arch = "riscv32"))]
    println!("Exiting Stopped state. Track: {:?}", ctx.current_track);
}

fn entry_loading(ctx: &mut MediaPlayerContext, _event: &MediaPlayerEvent) {
    #[cfg(not(target_arch = "riscv32"))]
    println!("Entering Loading state. Track: {:?}", ctx.current_track);
}

fn exit_loading(ctx: &mut MediaPlayerContext, _event: &MediaPlayerEvent) {
    #[cfg(not(target_arch = "riscv32"))]
    println!("Exiting Loading state. Track: {:?}", ctx.current_track);
}

fn entry_playing(ctx: &mut MediaPlayerContext, _event: &MediaPlayerEvent) {
    #[cfg(not(target_arch = "riscv32"))]
    println!("Entering Playing state. Track: {:?}", ctx.current_track);
}

fn exit_playing(ctx: &mut MediaPlayerContext, _event: &MediaPlayerEvent) {
    #[cfg(not(target_arch = "riscv32"))]
    println!("Exiting Playing state. Track: {:?}", ctx.current_track);
}

fn action_volume_up(ctx: &mut MediaPlayerContext, _event: &MediaPlayerEvent) {
    if ctx.volume < 100 {
        ctx.volume += 1;
    }
}

fn action_volume_down(ctx: &mut MediaPlayerContext, _event: &MediaPlayerEvent) {
    if ctx.volume > 0 {
        ctx.volume -= 1;
    }
}

statechart! {
    name: MediaPlayer,
    context: MediaPlayerContext,
    event: MediaPlayerEvent,
    initial: Stopped,

    state Stopped {
        entry: entry_stopped;
        exit: exit_stopped;
        on MediaPlayerEvent::Play [guard is_track_loaded] => Playing [action do_play];
        on LoadTrack { path: _ } [guard guard_for_load_track] => Loading [action action_for_load_track];
    }

    state Loading {
        entry: entry_loading;
        exit: exit_loading;
        on MediaPlayerEvent::Play [guard is_track_loaded] => Playing [action do_play];
    }

    state Playing {
        entry: entry_playing;
        exit: exit_playing;
        on MediaPlayerEvent::Stop => Stopped [action do_stop];
        on MediaPlayerEvent::VolumeUp => Playing [action action_volume_up];
        on MediaPlayerEvent::VolumeDown => Playing [action action_volume_down];
    }
}

#[cfg(not(target_arch = "riscv32"))] // Only include main for non-RISC-V targets
fn main() {
    let player_context = MediaPlayerContext {
        current_track: None, // Some("initial_track.mp3".to_string()),
        volume: 50,
    };
    let mut player = MediaPlayer::new(player_context.clone(), &MediaPlayerEvent::default());

    println!("Initial state: {:?}", player.state());
    println!("Initial context: {:?}", player.context());

    println!("Sending Stop (should do nothing if already stopped or no track):");
    let result = player.send(&MediaPlayerEvent::Stop);
    println!("Result: {result:?}");
    println!("State after Stop: {:?}", player.state());

    println!("Sending Play (should fail guard if no track):");
    let result = player.send(&MediaPlayerEvent::Play);
    println!("Result: {result:?}");
    println!("State after Play (no track): {:?}", player.state());

    // Uncommented test code for LoadTrack event pattern matching
    println!("Sending LoadTrack (empty path, should fail guard):");
    player.send(&MediaPlayerEvent::LoadTrack {
        path: String::try_from("").unwrap(),
    });
    println!("State after LoadTrack (empty path): {:?}", player.state());

    println!("Sending LoadTrack (invalid path, should fail guard):");
    player.send(&MediaPlayerEvent::LoadTrack {
        path: String::try_from("invalidtrack").unwrap(),
    });
    println!("State after LoadTrack (invalid path): {:?}", player.state());

    println!("Sending LoadTrack (valid path):");
    player.send(&MediaPlayerEvent::LoadTrack {
        path: String::try_from("valid_track.mp3").unwrap(),
    });
    println!("State after LoadTrack: {:?}", player.state());
    println!("Context after LoadTrack: {:?}", player.context());

    // The following Play will likely always fail the guard now since no track is loaded.
    // This is fine for this test, as we are checking for E0533.
    println!("Sending Play (track was never loaded):");
    let result = player.send(&MediaPlayerEvent::Play);
    println!("Result: {result:?}");
    println!("State after Play: {:?}", player.state());

    println!("Sending Stop:");
    let result = player.send(&MediaPlayerEvent::Stop);
    println!("Result: {result:?}");
    println!("State after Stop: {:?}", player.state());

    /*
    // Test the self-transition with pattern
    println!("Sending LoadTrack again (in Stopped state):");
    player.send(&MediaPlayerEvent::LoadTrack {
        path: String::try_from("another_track.mp4").unwrap(),
    });
    println!("State after LoadTrack: {:?}", player.state());
    println!("Context after LoadTrack: {:?}", player.context());

    println!("Sending Play (after loading 'another_track.mp4'):");
    player.send(&MediaPlayerEvent::Play);
    println!("State after Play: {:?}", player.state());
    */
}

// Panic handler is provided by panic_halt crate for RISC-V target

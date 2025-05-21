#![cfg_attr(target_arch = "riscv32", no_std)]
#![cfg_attr(target_arch = "riscv32", no_main)]

// This example is primarily for host testing for now.
// no_std string handling will be refined later.

#[cfg(target_arch = "riscv32")]
use panic_halt as _;

use lit_bit_core::StateMachine;
use lit_bit_macro::statechart;

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
pub enum MediaPlayerEvent {
    #[default]
    Play,
    Stop,
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
        println!("[Action] do_play - Track: {:?}", track);
    }
}

fn do_stop(_context: &mut MediaPlayerContext, _event: &MediaPlayerEvent) {
    println!("Stopping playback.");
}

fn is_track_loaded(context: &MediaPlayerContext, _event: &MediaPlayerEvent) -> bool {
    let loaded = context.current_track.is_some();
    println!("[Guard] is_track_loaded? {loaded}");
    loaded
}

fn guard_for_load_track(_context: &MediaPlayerContext, event: &MediaPlayerEvent) -> bool {
    if let MediaPlayerEvent::LoadTrack { path } = event {
        let valid = !path.is_empty() && path.contains('.');
        println!(
            "[Guard] guard_for_load_track ('{}')? {}",
            path.as_str(),
            valid
        );
        return valid;
    }
    false
}

fn action_for_load_track(context: &mut MediaPlayerContext, event: &MediaPlayerEvent) {
    if let MediaPlayerEvent::LoadTrack { path } = event {
        println!("[Action] action_for_load_track - Path: {}", path.as_str());
        context.current_track = Some(path.clone()); // Clone the heapless String
    } else {
        println!("[Action] action_for_load_track called with unexpected event type");
    }
}

fn entry_stopped(ctx: &mut MediaPlayerContext, _event: &MediaPlayerEvent) {
    println!("Entering Stopped state. Track: {:?}", ctx.current_track);
}

fn exit_stopped(ctx: &mut MediaPlayerContext, _event: &MediaPlayerEvent) {
    println!("Exiting Stopped state. Track: {:?}", ctx.current_track);
}

fn entry_loading(ctx: &mut MediaPlayerContext, _event: &MediaPlayerEvent) {
    println!("Entering Loading state. Track: {:?}", ctx.current_track);
}

fn exit_loading(ctx: &mut MediaPlayerContext, _event: &MediaPlayerEvent) {
    println!("Exiting Loading state. Track: {:?}", ctx.current_track);
}

fn entry_playing(ctx: &mut MediaPlayerContext, _event: &MediaPlayerEvent) {
    println!("Entering Playing state. Track: {:?}", ctx.current_track);
}

fn exit_playing(ctx: &mut MediaPlayerContext, _event: &MediaPlayerEvent) {
    println!("Exiting Playing state. Track: {:?}", ctx.current_track);
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
        on MediaPlayerEvent::LoadTrack { path: _ } [guard guard_for_load_track] => Loading [action action_for_load_track];
    }

    state Loading {
        entry: entry_loading;
        exit: exit_loading;
        on MediaPlayerEvent::Play => Playing [action do_play];
    }

    state Playing {
        entry: entry_playing;
        exit: exit_playing;
        on MediaPlayerEvent::Stop => Stopped [action do_stop];
        on MediaPlayerEvent::VolumeUp => Playing;
        on MediaPlayerEvent::VolumeDown => Playing;
    }
}

fn main() {
    let player_context = MediaPlayerContext {
        current_track: None, // Some("initial_track.mp3".to_string()),
        volume: 50,
    };
    let mut player = MediaPlayer::new(player_context.clone());

    println!("Initial state: {:?}", player.state());
    println!("Initial context: {:?}", player.context());

    println!("Sending Stop (should do nothing if already stopped or no track):");
    player.send(&MediaPlayerEvent::Stop);
    println!("State after Stop: {:?}", player.state());

    println!("Sending Play (should fail guard if no track):");
    player.send(&MediaPlayerEvent::Play);
    println!("State after Play (no track): {:?}", player.state());

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

    println!("Sending Play (should now succeed):");
    player.send(&MediaPlayerEvent::Play);
    println!("State after Play: {:?}", player.state());

    println!("Sending Stop:");
    player.send(&MediaPlayerEvent::Stop);
    println!("State after Stop: {:?}", player.state());

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
}

#[cfg(target_arch = "riscv32")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

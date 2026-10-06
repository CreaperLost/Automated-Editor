#[cfg(not(windows))]
use std::{ffi::c_void, ptr::NonNull};
#[cfg(all(target_os = "macos", not(stub_swift_ffi)))]
extern "C" {
    fn aeroshoot_audio_open() -> *mut c_void;
    fn aeroshoot_audio_close(handle: *mut c_void);
    fn aeroshoot_audio_queue(handle: *mut c_void, pcm: *const i16, frames: i32) -> i32;
    fn aeroshoot_audio_play(handle: *mut c_void);
    fn aeroshoot_audio_position(handle: *mut c_void) -> i64;
}
#[cfg(windows)]
pub use super::audio_cpal::AudioOutput;

#[cfg(not(windows))]
pub struct AudioOutput {
    #[allow(dead_code)]
    handle: NonNull<c_void>,
}
// Calls are serialized by the playback mutex; no callbacks retain the Rust owner.
#[cfg(not(windows))]
unsafe impl Send for AudioOutput {}
#[cfg(not(windows))]
impl AudioOutput {
    pub fn new() -> Result<Self, String> {
        #[cfg(all(target_os = "macos", not(stub_swift_ffi)))]
        unsafe {
            return NonNull::new(aeroshoot_audio_open())
                .map(|handle| Self { handle })
                .ok_or("Could not start the audio output device".into());
        }
        #[cfg(not(all(target_os = "macos", not(stub_swift_ffi))))]
        Err("Audio playback is not supported on this platform".into())
    }
    pub fn queue(&mut self, pcm: &[i16]) -> Result<(), String> {
        if pcm.is_empty() || pcm.len() % 2 != 0 || pcm.len() / 2 > crate::media::audio::CHUNK_FRAMES
        {
            return Err("Invalid playback PCM chunk".into());
        }
        #[cfg(all(target_os = "macos", not(stub_swift_ffi)))]
        unsafe {
            if aeroshoot_audio_queue(self.handle.as_ptr(), pcm.as_ptr(), (pcm.len() / 2) as i32)
                != 0
            {
                return Err("Audio scheduling failed".into());
            }
        }
        Ok(())
    }
    pub fn play(&mut self) {
        #[cfg(all(target_os = "macos", not(stub_swift_ffi)))]
        unsafe {
            aeroshoot_audio_play(self.handle.as_ptr());
        }
    }
    /// This output plays at normal speed only.
    pub fn set_speed(&mut self, speed: f64) -> bool {
        speed == 1.0
    }
    pub fn position_frames(&self) -> Result<u64, String> {
        #[cfg(all(target_os = "macos", not(stub_swift_ffi)))]
        unsafe {
            return u64::try_from(aeroshoot_audio_position(self.handle.as_ptr()))
                .map_err(|_| "Audio device stopped".into());
        }
        #[cfg(not(all(target_os = "macos", not(stub_swift_ffi))))]
        Err("No audio output".into())
    }
}
#[cfg(not(windows))]
impl Drop for AudioOutput {
    fn drop(&mut self) {
        #[cfg(all(target_os = "macos", not(stub_swift_ffi)))]
        unsafe {
            aeroshoot_audio_close(self.handle.as_ptr());
        }
    }
}

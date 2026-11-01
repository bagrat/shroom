//! The microphone as an audio source, through cpal (pure Rust over CoreAudio — no Swift,
//! never ffmpeg's avfoundation input, POC-LEARNINGS §3).
//!
//! Choosing a device follows the POC's hard lesson and N39: never auto-pick an iPhone's
//! Continuity mic (it drops audio and hitches video); prefer what the user picked, then the
//! system default, then the built-in mic.

use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde::Serialize;
use shroom_record::{AudioSink, AudioSource};

/// How a device is connected, from CoreAudio's transport type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Transport {
    BuiltIn,
    Usb,
    Bluetooth,
    /// An iPhone or iPad over Continuity — never chosen automatically.
    Continuity,
    Virtual,
    Aggregate,
    Other,
}

impl Transport {
    fn from_fourcc(code: u32) -> Transport {
        match &code.to_be_bytes() {
            b"bltn" => Transport::BuiltIn,
            b"usb " => Transport::Usb,
            b"blue" | b"blea" => Transport::Bluetooth,
            b"ccwd" | b"ccwl" | b"cccp" => Transport::Continuity,
            b"virt" => Transport::Virtual,
            b"aggr" | b"grup" => Transport::Aggregate,
            _ => Transport::Other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MicDevice {
    pub uid: String,
    pub name: String,
    pub transport: Transport,
}

/// The device to record from: the user's pick if it's here, else the system default
/// unless that's a Continuity device, else the built-in mic, else anything that isn't
/// Continuity. `None` when only a Continuity device is around.
pub fn choose<'a>(devices: &'a [MicDevice], picked: Option<&str>, default: Option<&str>) -> Option<&'a MicDevice> {
    let by_uid = |uid: &str| devices.iter().find(|d| d.uid == uid);
    if let Some(d) = picked.and_then(by_uid) {
        return Some(d);
    }
    if let Some(d) = default.and_then(by_uid).filter(|d| d.transport != Transport::Continuity) {
        return Some(d);
    }
    devices
        .iter()
        .find(|d| d.transport == Transport::BuiltIn)
        .or_else(|| devices.iter().find(|d| d.transport != Transport::Continuity))
}

#[repr(C)]
struct PropertyAddress {
    selector: u32,
    scope: u32,
    element: u32,
}

const fn fourcc(s: &[u8; 4]) -> u32 {
    u32::from_be_bytes(*s)
}

#[link(name = "CoreAudio", kind = "framework")]
unsafe extern "C" {
    fn AudioObjectGetPropertyData(
        object: u32,
        address: *const PropertyAddress,
        qualifier_size: u32,
        qualifier: *const c_void,
        data_size: *mut u32,
        data: *mut c_void,
    ) -> i32;
}

/// CoreAudio's transport type for the device with this UID.
fn transport_of(uid: &str) -> Transport {
    let cf_uid = cidre::cf::String::from_str(uid);
    let uid_ref: *const c_void = (&*cf_uid as *const cidre::cf::String).cast();
    let mut device: u32 = 0;
    let mut size = 4u32;
    let translate = PropertyAddress { selector: fourcc(b"uidd"), scope: fourcc(b"glob"), element: 0 };
    let status = unsafe {
        AudioObjectGetPropertyData(1, &translate, size_of::<*const c_void>() as u32, (&uid_ref as *const *const c_void).cast(), &mut size, (&mut device as *mut u32).cast())
    };
    if status != 0 || device == 0 {
        return Transport::Other;
    }
    let mut code: u32 = 0;
    let mut size = 4u32;
    let transport = PropertyAddress { selector: fourcc(b"tran"), scope: fourcc(b"glob"), element: 0 };
    let status = unsafe { AudioObjectGetPropertyData(device, &transport, 0, std::ptr::null(), &mut size, (&mut code as *mut u32).cast()) };
    if status != 0 { Transport::Other } else { Transport::from_fourcc(code) }
}

/// What the list reads of an input device: cpal's, or a test's.
trait Input {
    /// Its CoreAudio UID, past cpal's `host:` prefix.
    fn uid(&self) -> Option<String>;
    fn name(&self) -> Option<String>;
}

impl Input for cpal::Device {
    fn uid(&self) -> Option<String> {
        let uid = self.id().ok()?.to_string();
        Some(uid.split_once(':').map(|(_, u)| u.to_string()).unwrap_or(uid))
    }

    fn name(&self) -> Option<String> {
        self.description().ok().map(|x| x.name().to_string())
    }
}

/// One input device as the list shows it, or `None` when it can't be read whole: a device
/// going away mid-list answers its id, then fails its name. (Never cpal's `Display` for a
/// fallback: it is that same name read, and `to_string` panics on its error — the observer
/// thread died of it.)
fn listed(d: &impl Input, transport: impl FnOnce(&str) -> Transport) -> Option<MicDevice> {
    let uid = d.uid()?;
    let name = d.name()?;
    Some(MicDevice { transport: transport(&uid), uid, name })
}

/// Every input device, with its connection type.
pub fn input_devices() -> Vec<MicDevice> {
    let host = cpal::default_host();
    let Ok(devices) = host.input_devices() else { return Vec::new() };
    devices.filter_map(|d| listed(&d, transport_of)).collect()
}

/// The system's default input device's UID.
pub fn default_input_uid() -> Option<String> {
    let id = cpal::default_host().default_input_device()?.id().ok()?.to_string();
    Some(id.split_once(':').map(|(_, u)| u.to_string()).unwrap_or(id))
}

struct Live {
    _stream: cpal::Stream,
}
// cpal's CoreAudio stream is driven by CoreAudio's own thread; this handle is only kept
// alive and dropped, never used across threads concurrently.
unsafe impl Send for Live {}

pub struct MicSource {
    device: cpal::Device,
    config: cpal::StreamConfig,
    lost: Arc<AtomicBool>,
    live: Option<Live>,
    pub info: MicDevice,
}

impl MicSource {
    /// Opens the chosen device at its native rate (ffmpeg's AAC encoder takes it as is).
    pub fn open(info: MicDevice) -> Result<MicSource, String> {
        let host = cpal::default_host();
        let device = host
            .input_devices()
            .map_err(|e| e.to_string())?
            .find(|d| d.id().map(|id| id.to_string().ends_with(&info.uid)).unwrap_or(false))
            .ok_or("that microphone isn't connected")?;
        let supported = device.default_input_config().map_err(|e| e.to_string())?;
        if supported.sample_format() != cpal::SampleFormat::F32 {
            return Err(format!("unsupported sample format {:?}", supported.sample_format()));
        }
        Ok(MicSource { device, config: supported.config(), lost: Arc::default(), live: None, info })
    }

    /// Raised when the device goes away mid-take (N39's `mic-lost`).
    pub fn lost_flag(&self) -> Arc<AtomicBool> {
        self.lost.clone()
    }
}

impl AudioSource for MicSource {
    fn sample_rate(&self) -> u32 {
        self.config.sample_rate
    }

    fn start(&mut self, sink: AudioSink) -> Result<(), String> {
        let channels = self.config.channels.max(1) as usize;
        let lost = self.lost.clone();
        let mut mono: Vec<f32> = Vec::new();
        let stream = self
            .device
            .build_input_stream::<f32, _, _>(
                self.config,
                move |data: &[f32], _| {
                    if channels == 1 {
                        sink.push(data);
                    } else {
                        mono.clear();
                        mono.extend(data.chunks_exact(channels).map(|f| f.iter().sum::<f32>() / channels as f32));
                        sink.push(&mono);
                    }
                },
                move |_err| lost.store(true, Ordering::SeqCst),
                None,
            )
            .map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;
        self.live = Some(Live { _stream: stream });
        Ok(())
    }

    fn stop(&mut self) -> Result<(), String> {
        self.live = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev(uid: &str, t: Transport) -> MicDevice {
        MicDevice { uid: uid.into(), name: uid.into(), transport: t }
    }

    #[test]
    fn never_auto_picks_an_iphone() {
        let devices = [dev("iphone", Transport::Continuity), dev("builtin", Transport::BuiltIn)];
        assert_eq!(choose(&devices, None, Some("iphone")).unwrap().uid, "builtin");
        assert!(choose(&[dev("iphone", Transport::Continuity)], None, Some("iphone")).is_none());
    }

    #[test]
    fn the_users_pick_wins_even_if_it_is_an_iphone() {
        let devices = [dev("iphone", Transport::Continuity), dev("builtin", Transport::BuiltIn)];
        assert_eq!(choose(&devices, Some("iphone"), Some("builtin")).unwrap().uid, "iphone");
    }

    #[test]
    fn the_default_is_used_when_it_is_safe() {
        let devices = [dev("usb", Transport::Usb), dev("builtin", Transport::BuiltIn)];
        assert_eq!(choose(&devices, None, Some("usb")).unwrap().uid, "usb");
        assert_eq!(choose(&devices, Some("gone"), None).unwrap().uid, "builtin");
    }

    /// A device as cpal's CoreAudio one reads while it's there.
    struct Here;
    impl std::fmt::Display for Here {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("MacBook Pro Microphone")
        }
    }
    impl Input for Here {
        fn uid(&self) -> Option<String> {
            Some("BuiltInMicrophoneDevice".into())
        }
        fn name(&self) -> Option<String> {
            Some("MacBook Pro Microphone".into())
        }
    }

    /// A device leaving mid-list (AirPods going, a display's sound with display sleep), as
    /// cpal's CoreAudio one does it: its id still reads, its name no longer does, and its
    /// `Display` (that same name read) is an error — one `to_string` panics on.
    struct Leaving;
    impl std::fmt::Display for Leaving {
        fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            Err(std::fmt::Error)
        }
    }
    impl Input for Leaving {
        fn uid(&self) -> Option<String> {
            Some("AirPods-uid".into())
        }
        fn name(&self) -> Option<String> {
            None
        }
    }

    /// The mic list is re-read every 2 s on the observer thread; a device leaving under it is
    /// left out of that read (the next one has it gone), never a panic that ends the thread.
    #[test]
    fn a_mic_leaving_mid_list_is_left_out() {
        assert_eq!(listed(&Leaving, |_| Transport::Bluetooth), None);
        let here = listed(&Here, |_| Transport::BuiltIn).unwrap();
        assert_eq!((here.uid.as_str(), here.name.as_str(), here.transport), ("BuiltInMicrophoneDevice", "MacBook Pro Microphone", Transport::BuiltIn));
    }

    #[test]
    fn reads_transport_codes() {
        assert_eq!(Transport::from_fourcc(fourcc(b"bltn")), Transport::BuiltIn);
        assert_eq!(Transport::from_fourcc(fourcc(b"ccwl")), Transport::Continuity);
        assert_eq!(Transport::from_fourcc(fourcc(b"usb ")), Transport::Usb);
    }
}

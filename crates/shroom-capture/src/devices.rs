//! Microphone and camera access, through AVFoundation.

use cidre::av;

/// Where macOS stands on a device for this app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Access {
    NotAsked,
    Restricted,
    Denied,
    Granted,
}

pub fn mic_access() -> Access {
    access(av::MediaType::audio())
}

pub fn camera_access() -> Access {
    access(av::MediaType::video())
}

/// Shows macOS's prompt the first time; afterwards returns the standing answer.
pub fn request_mic() -> bool {
    request(av::MediaType::audio())
}

/// Shows macOS's prompt the first time; afterwards returns the standing answer.
pub fn request_camera() -> bool {
    request(av::MediaType::video())
}

fn access(media_type: &av::MediaType) -> Access {
    use av::capture::device::AuthorizationStatus as Status;
    match av::CaptureDevice::authorization_status_for_media_type(media_type) {
        Ok(Status::Authorized) => Access::Granted,
        Ok(Status::Denied) => Access::Denied,
        Ok(Status::Restricted) => Access::Restricted,
        _ => Access::NotAsked,
    }
}

fn request(media_type: &av::MediaType) -> bool {
    futures::executor::block_on(av::CaptureDevice::request_access_for_media_type(media_type))
        .unwrap_or(false)
}

/// A camera: its id, its human name, its model, whether it's the Mac's own, and whether
/// it's an iPhone or iPad over Continuity (never chosen automatically — N39).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Camera {
    pub uid: String,
    pub name: String,
    pub model: String,
    pub builtin: bool,
    pub continuity: bool,
}

/// What macOS says about one camera, before shroom decides what it is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CameraFacts<'a> {
    /// Its device type is the built-in wide-angle camera.
    pub builtin_type: bool,
    /// Its device type is `AVCaptureDeviceTypeContinuityCamera` — only reported to apps that
    /// opt in with `NSCameraUseContinuityCameraDeviceType`; otherwise an iPhone arrives as
    /// `External`.
    pub continuity_type: bool,
    /// `AVCaptureDevice.isContinuityCamera` — the answer that holds whatever the type says.
    pub is_continuity_camera: bool,
    /// `AVCaptureDevice.modelID`, e.g. `iPhone14,4`.
    pub model: &'a str,
}

/// Continuity is any of the three signals — the flag, the type, or an iPhone/iPad model —
/// so a macOS that drops one still never hands shroom an iPhone by itself (B48).
pub fn classify(f: CameraFacts) -> (bool, bool) {
    let continuity = f.is_continuity_camera || f.continuity_type || f.model.starts_with("iPhone") || f.model.starts_with("iPad");
    (f.builtin_type && !continuity, continuity)
}

/// Every camera macOS offers right now: built-in, external, Continuity.
pub fn cameras() -> Vec<Camera> {
    use av::capture::device::{DiscoverySession, Pos, Type};
    // These two are weakly linked (newer macOS); cidre marks reading them unsafe.
    let (external, continuity) = unsafe { (Type::external(), Type::continuity_camera()) };
    let builtin = Type::built_in_wide_angle_camera();
    let mut kinds: Vec<&Type> = vec![builtin];
    kinds.extend(external);
    kinds.extend(continuity);
    let types = cidre::ns::Array::from_slice(&kinds);
    let session = DiscoverySession::with_device_types_media_and_pos(&types, Some(av::MediaType::video()), Pos::Unspecified);
    session
        .devices()
        .iter()
        .map(|d| {
            let model = d.model_id().to_string();
            let (builtin, continuity) = classify(CameraFacts {
                builtin_type: d.device_type() == builtin,
                continuity_type: continuity.is_some_and(|c| d.device_type() == c),
                is_continuity_camera: d.is_continuity_camera(),
                model: &model,
            });
            Camera { uid: d.unique_id().to_string(), name: d.localized_name().to_string(), model, builtin, continuity }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_iphone_reported_as_external_is_still_continuity() {
        // macOS 26 without the opt-in key: type External, flag set, model iPhone14,4.
        let f = CameraFacts { is_continuity_camera: true, model: "iPhone14,4", ..Default::default() };
        assert_eq!(classify(f), (false, true));
        // Any one signal alone is enough.
        assert_eq!(classify(CameraFacts { model: "iPhone14,4", ..Default::default() }), (false, true));
        assert_eq!(classify(CameraFacts { model: "iPad13,1", ..Default::default() }), (false, true));
        assert_eq!(classify(CameraFacts { continuity_type: true, ..Default::default() }), (false, true));
    }

    #[test]
    fn the_macs_own_camera_and_a_webcam_are_not_continuity() {
        assert_eq!(classify(CameraFacts { builtin_type: true, model: "FaceTime HD Camera", ..Default::default() }), (true, false));
        assert_eq!(classify(CameraFacts { model: "Logitech BRIO", ..Default::default() }), (false, false));
    }

    /// Runs against the real device list: whatever this Mac has, an iPhone or iPad is never
    /// taken for an ordinary camera (vacuous on a Mac with no iPhone nearby).
    #[test]
    fn real_cameras_never_pass_an_iphone_off_as_ordinary() {
        for c in cameras() {
            let looks_iphone = c.model.starts_with("iPhone") || c.model.starts_with("iPad") || c.name.contains("iPhone") || c.name.contains("iPad");
            if looks_iphone {
                assert!(c.continuity && !c.builtin, "{c:?}");
            }
            if c.builtin {
                assert!(!c.continuity, "{c:?}");
            }
        }
    }
}

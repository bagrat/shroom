// shroom — native microphone tap (`shroom --mic-tap`).
//
// THE POINT: capture the mic through Apple's native audio engine, NOT through
// ffmpeg's avfoundation audio demuxer. That demuxer's buffer handling corrupts the
// built-in mic — real digital splices, a ~decade-old, version-independent ffmpeg bug
// (see the capture-empirical-findings notes). AVAudioEngine reads the same mic
// crystal-clear. So the recorder runs THIS as a child, reads mono float32 (f32le)
// PCM off its stdout, buffers it, and feeds it to ffmpeg as a second input via a
// fifo. ffmpeg still owns screen + encode + segment + tee; only the audio source
// changes. This is a MODE of the shim binary (not a separate helper) so it shares
// the shim's ad-hoc signature, its microphone TCC grant, and its Info.plist mic
// usage string — one identity, one grant, one build.
//
//   shroom --mic-tap [--mic-device "<name>"]            # stream mono f32le on stdout
//   shroom --mic-tap --probe [--mic-device "<name>"]    # print `rate=<n>` and exit
//
// --mic-device points the engine at a specific input device by name (matched against
// the CoreAudio device list, case-insensitive substring — the picker's chosen mic).
// Absent or unmatched → the system default input. Deliberately NO resampling: the
// recorder reads the probed rate and tells ffmpeg the matching -ar, so pitch/speed
// are exact and no DSP sits between the mic and ffmpeg.

import AVFoundation
import CoreAudio
import Foundation

enum MicTap {
    // MARK: CoreAudio device lookup (mirrors dev/audio-prototypes/ratectl.swift)

    private static func allDevices() -> [AudioDeviceID] {
        var addr = AudioObjectPropertyAddress(
            mSelector: kAudioHardwarePropertyDevices,
            mScope: kAudioObjectPropertyScopeGlobal,
            mElement: kAudioObjectPropertyElementMain)
        var size: UInt32 = 0
        AudioObjectGetPropertyDataSize(AudioObjectID(kAudioObjectSystemObject), &addr, 0, nil, &size)
        let n = Int(size) / MemoryLayout<AudioDeviceID>.size
        var ids = [AudioDeviceID](repeating: 0, count: n)
        AudioObjectGetPropertyData(AudioObjectID(kAudioObjectSystemObject), &addr, 0, nil, &size, &ids)
        return ids
    }

    private static func deviceName(_ id: AudioDeviceID) -> String {
        var addr = AudioObjectPropertyAddress(
            mSelector: kAudioObjectPropertyName,
            mScope: kAudioObjectPropertyScopeGlobal,
            mElement: kAudioObjectPropertyElementMain)
        var s: CFString = "" as CFString
        var size = UInt32(MemoryLayout<CFString>.size)
        AudioObjectGetPropertyData(id, &addr, 0, nil, &size, &s)
        return s as String
    }

    private static func hasInput(_ id: AudioDeviceID) -> Bool {
        var addr = AudioObjectPropertyAddress(
            mSelector: kAudioDevicePropertyStreams,
            mScope: kAudioObjectPropertyScopeInput,
            mElement: kAudioObjectPropertyElementMain)
        var size: UInt32 = 0
        AudioObjectGetPropertyDataSize(id, &addr, 0, nil, &size)
        return size > 0
    }

    // Resolve a device-name substring to an INPUT device id, else nil (→ default input).
    private static func inputDevice(matching needle: String) -> AudioDeviceID? {
        let q = needle.lowercased()
        // Prefer an exact (case-insensitive) name match, else the first substring hit.
        let inputs = allDevices().filter { hasInput($0) }
        return inputs.first { deviceName($0).lowercased() == q }
            ?? inputs.first { deviceName($0).lowercased().contains(q) }
    }

    // MARK: engine

    // Build the engine, pointing its input at `device` when given. Reading inputFormat
    // AFTER setting the current device is what makes the format reflect that device.
    private static func makeEngine(device: String?) -> (AVAudioEngine, AVAudioFormat) {
        let engine = AVAudioEngine()
        let input = engine.inputNode   // instantiates the input unit
        if let device, let id = inputDevice(matching: device), let unit = input.audioUnit {
            var dev = id
            let st = AudioUnitSetProperty(
                unit, kAudioOutputUnitProperty_CurrentDevice, kAudioUnitScope_Global, 0,
                &dev, UInt32(MemoryLayout<AudioDeviceID>.size))
            if st != noErr {
                FileHandle.standardError.write("MICTAP: could not select \"\(device)\" (err \(st)); using default input\n".data(using: .utf8)!)
            }
        } else if let device {
            FileHandle.standardError.write("MICTAP: no input device matching \"\(device)\"; using default input\n".data(using: .utf8)!)
        }
        return (engine, input.inputFormat(forBus: 0))
    }

    // MARK: modes

    static func runAndExit(device: String?, probe: Bool) -> Never {
        let (engine, fmt) = makeEngine(device: device)
        let rate = Int(fmt.sampleRate)
        let ch = Int(fmt.channelCount)
        FileHandle.standardError.write("MICTAP rate=\(rate) ch=\(ch)\n".data(using: .utf8)!)

        if probe {
            print("rate=\(rate)")   // machine-parseable on stdout for the recorder
            exit(0)
        }

        let out = FileHandle.standardOutput
        // Tap channel 0 → mono. Built-in mic is 1ch; a multichannel device takes ch 0.
        engine.inputNode.installTap(onBus: 0, bufferSize: 4096, format: fmt) { buf, _ in
            guard let chans = buf.floatChannelData else { return }
            let n = Int(buf.frameLength)
            out.write(Data(bytes: chans[0], count: n * MemoryLayout<Float>.size))
        }

        let stop = {
            engine.stop()
            engine.inputNode.removeTap(onBus: 0)
            try? out.synchronize()
            exit(0)
        }
        // Drive stop off a DispatchSource so the write tap (a separate thread) can't
        // race a mid-buffer teardown.
        signal(SIGINT, SIG_IGN)
        signal(SIGTERM, SIG_IGN)
        let s1 = DispatchSource.makeSignalSource(signal: SIGINT, queue: .main)
        let s2 = DispatchSource.makeSignalSource(signal: SIGTERM, queue: .main)
        s1.setEventHandler { stop() }
        s2.setEventHandler { stop() }
        s1.resume(); s2.resume()

        do {
            try engine.start()
        } catch {
            FileHandle.standardError.write("MICTAP engine start failed: \(error)\n".data(using: .utf8)!)
            exit(1)
        }
        FileHandle.standardError.write("MICTAP capturing (mono f32le @\(rate)) — SIGTERM to stop\n".data(using: .utf8)!)
        dispatchMain()
    }
}

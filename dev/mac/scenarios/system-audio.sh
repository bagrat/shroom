# The Mac's own sound in a take (spec N47, DECISIONS B172): the dev app records two takes of
# the test pattern with the drives' beep as the voice and the Mac's sound mixed in — another
# process plays a 440 Hz burst half a second after each beep in take 0, nothing in take 1 —
# while the app plays a 2 kHz tone of its own all along, which must never be in a take.
# Ends with 2 when both sounds are there but an offset bar was missed on the runner.
SHROOM_SYSTEM_AUDIO_OUT="$MAC_OUT" node tests/e2e/system-audio-check.mjs 12

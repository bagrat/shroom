#!/bin/sh
# Makes the media fixtures beside this script (tests/peaks.rs reads them; nothing encodes at
# test time). Any stock ffmpeg with libx264 will do; the files are committed, so this runs
# only to change them.
#
#   tone.mp4         4 s: a 440 Hz tone at 0.8 for 2 s, then 2 s of silence; mono AAC
#                    48 kHz beside a tiny H.264 picture; faststart, like a take's
#                    preview.mp4 and the bucket's video.mp4
#   tone-frag.mp4    the same, fragmented the way a take's preview_<k>.mp4 is written
#                    (crates/shroom-encode/src/args.rs)
#   video-only.mp4   3 s of picture, no sound track
set -eu
cd "$(dirname "$0")"

picture() { echo "color=c=0x3366cc:s=64x36:r=10:d=$1"; }
TONE="aevalsrc=if(lt(t\,2)\,0.8*sin(2*PI*440*t)\,0):s=48000:d=4"
X264="-c:v libx264 -preset veryslow -crf 40 -pix_fmt yuv420p"
AAC="-c:a aac -b:a 32k -ac 1"

# shellcheck disable=SC2086
ffmpeg -hide_banner -loglevel error -y -f lavfi -i "$(picture 4)" -f lavfi -i "$TONE" \
  $X264 $AAC -shortest -map_metadata -1 -fflags +bitexact -movflags +faststart tone.mp4
# shellcheck disable=SC2086
ffmpeg -hide_banner -loglevel error -y -f lavfi -i "$(picture 4)" -f lavfi -i "$TONE" \
  $X264 $AAC -shortest -map_metadata -1 -fflags +bitexact -f mp4 \
  -movflags +frag_keyframe+empty_moov+delay_moov+default_base_moof -frag_duration 1000000 tone-frag.mp4
# shellcheck disable=SC2086
ffmpeg -hide_banner -loglevel error -y -f lavfi -i "$(picture 3)" \
  $X264 -map_metadata -1 -fflags +bitexact -movflags +faststart video-only.mp4
ls -l tone.mp4 tone-frag.mp4 video-only.mp4

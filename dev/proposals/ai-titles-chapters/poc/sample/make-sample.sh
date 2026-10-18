#!/bin/sh
# Builds an invented sample recording shaped like a shroom take: H.264 + AAC 128k in
# 6 s HLS-fMP4 pieces (init.mp4 + seg_NNNNN.m4s), from script.txt read by ffmpeg's flite,
# one paragraph at a time, slowed to a talking pace with a breath between paragraphs.
# Usage: make-sample.sh <out-dir> [script.txt]
set -eu
here=$(dirname "$0"); out=${1:?out dir}; script=${2:-$here/script.txt}; mkdir -p "$out"; rm -f "$out"/seg_*.m4s "$out"/p*.wav
: >"$out/list.txt"; n=0
grep -v '^$' "$script" | while IFS= read -r para; do
  n=$((n + 1)); printf '%s' "$para" | sed "s/'//g; s/:/ /g" >"$out/p$n.txt"
  ffmpeg -hide_banner -loglevel error -y -f lavfi -i "flite=textfile=$out/p$n.txt:voice=slt" \
    -af "atempo=0.82,apad=pad_dur=1.2" -ar 48000 -ac 1 "$out/p$n.wav"
  echo "file 'p$n.wav'" >>"$out/list.txt"
done
ffmpeg -hide_banner -loglevel error -y -f concat -safe 0 -i "$out/list.txt" "$out/voice.wav"
dur=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$out/voice.wav")
ffmpeg -hide_banner -loglevel error -y -f lavfi -i "testsrc2=size=1280x720:rate=30" -i "$out/voice.wav" -t "$dur" \
  -c:v libx264 -preset veryfast -b:v 3500k -pix_fmt yuv420p -g 60 -c:a aac -b:a 128k -ac 2 \
  -f hls -hls_segment_type fmp4 -hls_time 6 -hls_playlist_type vod -hls_fmp4_init_filename init.mp4 \
  -hls_segment_filename "$out/seg_%05d.m4s" "$out/stream.m3u8"
echo "$dur s, $(ls "$out"/seg_*.m4s | wc -l) pieces"

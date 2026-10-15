# Camera only with its preview dragged before Start (Bagrat's 0.1.10 checkpoint: the rectangle
# snapped back at Start, then the take died with nothing saved and nothing said). The preview
# must keep its place and window through the count and the take, and the take must survive
# Stop whole. The hosted Mac's camera is OBS's virtual camera on the test pattern.
bash dev/mac/camera.sh up || exit 1
node app/test/take-drive.mjs camera-drag

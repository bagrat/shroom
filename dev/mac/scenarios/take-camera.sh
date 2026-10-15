# A take with Camera as the source, kept: the hosted Mac's camera (dev/mac/camera.sh: OBS's
# virtual camera on the test pattern), then the real flow — config, Return, the count, five
# seconds, Stop — and the take's file and a frame of it in the evidence (camera-take.mp4,
# camera-take.png), the frame read for the pattern's bars (DECISIONS B121).
bash dev/mac/camera.sh up || exit 1
SHROOM_TAKE_OUT="$MAC_OUT" node app/test/take-drive.mjs camera-take

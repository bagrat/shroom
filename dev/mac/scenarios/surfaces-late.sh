# The surfaces drive with neither overlay panel made as shroom starts: each is made the
# late way the first time it shows — the path a failed start-up takes — and every check of
# `surfaces` holds there too. Strict, as `surfaces` is.
node app/test/surfaces-drive.mjs --fullscreen --strict --late-panels --report="$MAC_OUT/surfaces.json"

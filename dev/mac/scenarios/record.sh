# The record check: the dev app covers the display with the test pattern, records two takes
# with a pause through its real path, and every frame's counter is read back. Ends with 2
# when the recording is whole but a timing bar was missed — on a hosted runner (three
# virtual cores) that is the runner's speed, and a Mac decides it.
node tests/e2e/record-check.mjs 6

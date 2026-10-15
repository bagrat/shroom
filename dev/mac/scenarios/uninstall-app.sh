# Uninstall shroom… in the app, for real (DECISIONS B135): a throwaway copy of the dev app
# walks Settings · General → Uninstall, quits, and its helper removes it and its home;
# macOS holds no grant for the dev id after. It resets those grants: name it last.
cd app && sh test/uninstall-drive.sh

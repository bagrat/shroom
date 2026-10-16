# shroom, installed via Homebrew: `brew install --cask bagrat/shroom/shroom`.
#
# The tap (github.com/bagrat/homebrew-shroom) holds this file with the latest release's
# version and SHA-256 filled in by installer/homebrew/render-cask.sh — its own workflow
# renders it from this file on `main` and the release's manifest.txt. The version and
# fingerprint below are only the newest when this file last changed; the tap's are live.
#
# It does what installer/install.sh does, the Homebrew way: the release's zip, checked
# against its fingerprint; placed in ~/Applications (no admin password, and the same copy
# the install line and the app's own updater keep); taken only if it is signed as
# am.shroom by the release certificate (DECISIONS B125). That signature is self-signed, not
# notarized, so macOS would refuse to open a copy marked as downloaded — the install line's
# curl never marks it, and this removes the mark Homebrew adds, after the signature checks.
cask "shroom" do
  version "0.1.16"
  sha256 "9d42c32a02b2e3a8f3e19260bd9872806e514fc3b438f4fd9ec7d64ca95bff88"

  url "https://github.com/bagrat/shroom/releases/download/v#{version}/shroom-#{version}-arm64.zip"
  name "shroom"
  desc "Screen recordings that live in your own Cloudflare account"
  homepage "https://shroom.am/"

  livecheck do
    url "https://github.com/bagrat/shroom/releases/latest/download/manifest.txt"
    regex(/^version=(\d+(?:\.\d+)+)$/i)
  end

  # The app updates itself (DECISIONS B31): `brew upgrade` leaves it be unless --greedy.
  auto_updates true
  depends_on arch: :arm64
  depends_on macos: ">= :sonoma"

  app "shroom.app", target: "~/Applications/shroom.app"

  postflight do
    # The release certificate's SHA-1 (tools/release-identity.sha1; the installer's
    # RELEASE_LEAF; installer/test/cask.test.mjs holds the three equal).
    leaf = "8344cbe040722ea0a5632320c1d0d4348eaa3af3"
    placed = File.expand_path("~/Applications/shroom.app")
    signed = system_command("/usr/bin/codesign",
                            args:         ["--verify", "--deep", "--strict", "-R",
                                           "=identifier \"am.shroom\" and certificate leaf = H\"#{leaf}\"",
                                           placed],
                            print_stderr: false).success?
    unless signed
      FileUtils.rm_rf(placed)
      raise "shroom's signature didn't verify, so nothing was installed. If it repeats, tell us: " \
            "https://github.com/bagrat/shroom/issues/new — no GitHub? hi@shroom.am works too."
    end
    system_command "/usr/bin/xattr", args: ["-dr", "com.apple.quarantine", placed]
  end

  uninstall quit: "am.shroom"

  # What `install.sh --uninstall` removes beside the app (DECISIONS B135).
  zap trash: [
    "~/.shroom",
    "~/Library/Caches/am.shroom",
    "~/Library/HTTPStorages/am.shroom",
    "~/Library/Preferences/am.shroom.plist",
    "~/Library/Saved Application State/am.shroom.savedState",
    "~/Library/WebKit/am.shroom",
  ]
end

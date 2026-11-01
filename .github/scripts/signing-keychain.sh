#!/bin/sh
# A GitHub runner's signing keychain (DECISIONS B102): what the tag workflow signs a release
# from, and what CI rehearses on every push so a tag never meets it for the first time.
#
# RUNNER ONLY. It puts a keychain on the search list and a certificate into the machine's
# trust settings — things nobody wants done to their own Mac — so it refuses to run anywhere
# but a GitHub Actions runner, which is thrown away after the job.
#
#   signing-keychain.sh import          from SIGNING_P12_BASE64 and SIGNING_P12_PASSWORD (the
#                                       environment, never the command line): a throwaway
#                                       keychain holding the identity (its key can't be
#                                       exported), usable by codesign without a prompt and
#                                       trusted for code signing on this runner. Says `identity=<name>` and `leaf=<sha1>` to
#                                       $GITHUB_OUTPUT — both public: they are in every release.
#   signing-keychain.sh delete          everything `import` made, gone (an always() step)
#   signing-keychain.sh scratch <path>  an empty, unlocked keychain file — where the rehearsal
#                                       lets tools/make-release-identity.sh make its identity
#   signing-keychain.sh drop <path>     that keychain, gone
#
# Why trust at all: codesign signs only with an identity whose certificate this Mac trusts
# for code signing (it answers "no identity found" otherwise — probed 2026-10-12), and a
# self-signed certificate is trusted only where someone said so. The people who install
# shroom never need that: macOS checks a signature against the certificate inside it
# (RESEARCH §5b, E2). Only the Mac that signs does.
#
# No secret is printed: the .p12's password is read from the environment by openssl, the
# keychain gets a random password nobody learns, and the key leaves its file for the
# keychain under a one-time password.
set -eu

fail() { echo "::error::$*"; exit 1; }
[ "${GITHUB_ACTIONS:-}" = true ] && [ -n "${RUNNER_TEMP:-}" ] || { echo "signing-keychain: runs on a GitHub Actions runner only — never on your own Mac" >&2; exit 1; }

OPENSSL=/usr/bin/openssl
dir=$RUNNER_TEMP/shroom-signing
kc=$RUNNER_TEMP/shroom-signing.keychain-db
umask 077

# The user's search list, one path per line.
search_list() { security list-keychains -d user | sed -e 's/^ *"//' -e 's/" *$//'; }

new_keychain() { # <path> — empty, unlocked, never locking itself
  pw=$("$OPENSSL" rand -hex 24)
  security create-keychain -p "$pw" "$1"
  security set-keychain-settings "$1"
  security unlock-keychain -p "$pw" "$1"
}

# Run one command, but never wait on it longer than <seconds>: some `security` commands stop
# to ask a person at the screen, and a runner has none. Its output goes to <log> (or
# nowhere), never to the step's own pipe — a command left behind must not hold the step open.
# (SIGNING_WAIT shortens the wait, for testing this script with stand-in commands.)
within() { # <seconds> <log|-> <command…>
  secs=$1; log=$2; shift 2
  [ "$log" != - ] || log=/dev/null
  "$@" </dev/null >>"$log" 2>&1 &
  cmd=$!
  ( sleep "$secs"; kill "$cmd" 2>/dev/null; sleep 2; kill -9 "$cmd" 2>/dev/null ) </dev/null >/dev/null 2>&1 &
  dog=$!
  rc=0
  wait "$cmd" 2>/dev/null || rc=$?
  kill "$dog" 2>/dev/null || true
  wait "$dog" 2>/dev/null || true
  return "$rc"
}

# Run one command with an authorization right allowed — the right that normally needs a
# person at the screen is granted for the one command, then put back.
grant() { # <log> <right> <command…>
  glog=$1; right=$2; shift 2
  sudo -n security authorizationdb read "$right" >"$dir/right.plist" 2>>"$glog" || true
  sudo -n security authorizationdb write "$right" allow >>"$glog" 2>&1 || true
  within "${SIGNING_WAIT:-60}" "$glog" "$@" || true
  if [ -s "$dir/right.plist" ]; then
    sudo -n security authorizationdb write "$right" <"$dir/right.plist" >>"$glog" 2>&1 || true
  fi
}

case "${1-}" in
  scratch)
    [ $# -eq 2 ] || fail "usage: signing-keychain.sh scratch <path>"
    new_keychain "$2"
    ;;

  drop)
    [ $# -eq 2 ] || fail "usage: signing-keychain.sh drop <path>"
    security delete-keychain "$2"
    ;;

  import)
    [ -n "${SIGNING_P12_BASE64:-}" ] || fail "No signing certificate: the release environment has no RELEASE_CERT_P12_BASE64 secret — docs/RELEASING.md, one-time setup."
    [ -n "${SIGNING_P12_PASSWORD:-}" ] || fail "No certificate password: the release environment has no RELEASE_CERT_PASSWORD secret — docs/RELEASING.md, one-time setup."
    [ ! -e "$kc" ] || fail "a signing keychain is already here"
    mkdir -p "$dir"

    printf '%s' "$SIGNING_P12_BASE64" | base64 --decode >"$dir/in.p12" 2>/dev/null || fail "RELEASE_CERT_P12_BASE64 isn't the .p12 as base64 — set it from the .p12.base64 file again."
    "$OPENSSL" pkcs12 -in "$dir/in.p12" -passin env:SIGNING_P12_PASSWORD -nodes -out "$dir/all.pem" >/dev/null 2>&1 \
      || { rm -f "$dir/in.p12" "$dir/all.pem"; fail "The certificate doesn't open with its password: RELEASE_CERT_P12_BASE64 and RELEASE_CERT_PASSWORD don't belong together."; }
    rm -f "$dir/in.p12"
    "$OPENSSL" x509 -in "$dir/all.pem" -out "$dir/cert.pem"
    name=$("$OPENSSL" x509 -in "$dir/cert.pem" -noout -subject -nameopt multiline | sed -n 's/^ *commonName *= *//p')
    leaf=$("$OPENSSL" x509 -in "$dir/cert.pem" -noout -fingerprint -sha1 | sed -e 's/^.*=//' -e 's/://g' | tr 'A-F' 'a-f')
    [ -n "$name" ] && [ ${#leaf} -eq 40 ] || fail "couldn't read the certificate's name and SHA-1"
    "$OPENSSL" x509 -in "$dir/cert.pem" -noout -checkend 2592000 >/dev/null || fail "The signing certificate expires within 30 days — a release signed with a new one costs every user their Screen Recording permission. docs/RELEASING.md."

    # Into the keychain under a one-time password, in the ciphers `security import` reads.
    "$OPENSSL" rand -hex 24 | tr -d '\n' >"$dir/once"
    "$OPENSSL" pkcs12 -export -in "$dir/all.pem" -name "$name" -keypbe PBE-SHA1-3DES -certpbe PBE-SHA1-3DES -macalg sha1 \
      -out "$dir/once.p12" -passout "file:$dir/once" >/dev/null 2>&1 || fail "couldn't prepare the identity for the keychain"
    rm -f "$dir/all.pem"
    kpw=$("$OPENSSL" rand -hex 24)
    security create-keychain -p "$kpw" "$kc"
    security set-keychain-settings "$kc"
    security unlock-keychain -p "$kpw" "$kc"
    # Non-extractable (-x): the key can sign here, and can never be exported from here.
    security import "$dir/once.p12" -k "$kc" -f pkcs12 -P "$(cat "$dir/once")" -x -T /usr/bin/codesign >/dev/null \
      || fail "the keychain didn't take the identity"
    rm -f "$dir/once.p12" "$dir/once"
    # codesign may use the key without asking anyone — Apple's own tools only.
    security set-key-partition-list -S apple-tool:,apple: -s -k "$kpw" "$kc" >/dev/null
    # On the search list, ahead of the runner's own keychains: where codesign looks a name up.
    rest=$(search_list | grep -vxF "$kc" || true)
    old_ifs=$IFS
    IFS='
'
    # shellcheck disable=SC2086
    set -- "$kc" $rest
    IFS=$old_ifs
    security list-keychains -d user -s "$@"

    # Trusted for code signing, on this runner only. The machine's trust settings first (the
    # road hosted runners are known to take: the authorization that normally needs a person
    # at the screen is granted for the one command, then put back); the user's as a second try.
    trusted() { security find-identity -v -p codesigning "$kc" 2>/dev/null | grep -qF "\"$name\""; }
    : >"$dir/trust.log"
    trusted || grant "$dir/trust.log" com.apple.trust-settings.admin sudo -n security add-trusted-cert -d -r trustRoot -p codeSign -k /Library/Keychains/System.keychain "$dir/cert.pem"
    trusted || grant "$dir/trust.log" com.apple.trust-settings.user security add-trusted-cert -r trustRoot -p codeSign -k "$kc" "$dir/cert.pem"
    if ! trusted; then
      cat "$dir/trust.log"
      security find-identity -p codesigning "$kc" | sed 's/[0-9A-F]\{40\}/…/' || true
      fail "This runner won't trust the certificate for code signing, so codesign can't sign with it — nothing was built. docs/RELEASING.md, “If signing fails on the runner”."
    fi

    { echo "identity=$name"; echo "leaf=$leaf"; } >>"$GITHUB_OUTPUT"
    echo "The signing identity is ready: $name (certificate $leaf)."
    ;;

  delete)
    # The key first: whatever happens after, it is gone. The trust last, best effort and on
    # a clock — the runner is thrown away anyway, and removing a trust setting can stop to
    # ask a person at the screen (CI's first run hung right here, 2026-10-12: the removal
    # ran without the right `import` grants itself, and waited for nobody).
    if [ -e "$kc" ]; then
      within "${SIGNING_WAIT:-60}" - security delete-keychain "$kc" || true
      rm -f "$kc"
    fi
    if [ -f "$dir/cert.pem" ]; then
      grant /dev/null com.apple.trust-settings.admin sudo -n security remove-trusted-cert -d "$dir/cert.pem"
      grant /dev/null com.apple.trust-settings.user security remove-trusted-cert "$dir/cert.pem"
    fi
    rm -rf "$dir"
    echo "The signing keychain is gone."
    ;;

  *) fail "usage: signing-keychain.sh import | delete | scratch <path> | drop <path>" ;;
esac

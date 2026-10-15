#!/bin/sh
# Makes the release's signing identity — once (DECISIONS B102; RESEARCH §5b, E1/E2).
#
# shroom's releases are signed with a self-signed code-signing certificate that is Bagrat's
# alone: no Apple Developer ID, no notarization (the app arrives by curl, so it is never
# quarantined). macOS keeps a person's Screen Recording permission by the app's identifier
# and this certificate — `identifier "am.shroom" and certificate leaf = H"…"` — so THE SAME
# CERTIFICATE SIGNS EVERY RELEASE, FOREVER. A release signed with another one is a new app
# to macOS: everyone is asked for Screen Recording again. So this script makes one, refuses
# to make a second of the same name, and says how to keep it.
#
#   tools/make-release-identity.sh [name] [--keychain <path>] [--out <dir>] [--days <n>]
#   tools/make-release-identity.sh [name] [--keychain <path>] --leaf
#
#   name        the identity's name, as codesign will know it (default: shroom release)
#   --keychain  the keychain file that gets the identity (default: your login keychain)
#   --out       where the files below go; made if missing, mode 700; never inside a git
#               checkout (default: ~/.shroom/release-identity)
#   --days      how long the certificate is valid (default: 7300 — twenty years)
#   --leaf      make nothing: print the SHA-1 of the certificate of that name already in
#               the keychain — the value of tools/release-identity.sha1
#
# What it leaves in <out> (names from the identity's: `shroom release` → shroom-release.…):
#
#   ….p12          the certificate and its private key, password-protected — the backup
#   ….p12.base64   the same, as one line of text — the GitHub secret RELEASE_CERT_P12_BASE64
#   ….password     the .p12's password                — the GitHub secret RELEASE_CERT_PASSWORD
#   ….cer          the certificate alone (public; it travels inside every release anyway)
#   ….sha1         the certificate's SHA-1 — the pin, committed as tools/release-identity.sha1
#
# The private key and the password are never printed and never on a command line: the key
# is made in a private temp dir that is removed on the way out, the password is read from
# its file. (The keychain import alone needs a password on its command line — it gets a
# one-time one, for a copy that is deleted with the temp dir.)
#
# It never replaces anything: a certificate of that name already in the keychain, or any of
# the files already in <out>, stops it before a key is made. It changes no trust setting —
# macOS trusts nothing new because of this script. (codesign signs only with an identity
# this Mac trusts for code signing; the release workflow sets that up on its own throwaway
# runner. To sign with it on this Mac too, see docs/RELEASING.md.)
set -eu

fail() { echo "make-release-identity: $*" >&2; exit 1; }
usage() { echo "usage: tools/make-release-identity.sh [name] [--keychain <path>] [--out <dir>] [--days <n>] | [name] [--keychain <path>] --leaf" >&2; exit 64; }

name="shroom release"
keychain=
out=
days=7300
leaf_only=
named=
while [ $# -gt 0 ]; do
  case "$1" in
    --keychain) [ $# -ge 2 ] || usage; keychain=$2; shift 2 ;;
    --out) [ $# -ge 2 ] || usage; out=$2; shift 2 ;;
    --days) [ $# -ge 2 ] || usage; days=$2; shift 2 ;;
    --leaf) leaf_only=1; shift ;;
    -*) usage ;;
    *) [ -z "$named" ] || usage; name=$1; named=1; shift ;;
  esac
done

# A name codesign, a file name and a line of JSON can all carry.
case "$name" in
  ""|-*|*[!A-Za-z0-9\ ._-]*) fail "the name can hold letters, digits, spaces, dots, dashes and underscores — not '$name'" ;;
esac
case "$days" in ""|*[!0-9]*) fail "--days takes a whole number of days" ;; esac
[ "$days" -ge 365 ] || fail "a release identity that expires within a year would cost every user their permission — give it years (--days 7300)"
[ "$(uname -s)" = Darwin ] || fail "runs on macOS only"
OPENSSL=/usr/bin/openssl
for tool in "$OPENSSL" security base64 shasum; do
  command -v "$tool" >/dev/null 2>&1 || fail "needs $tool, which isn't on this Mac"
done

if [ -z "$keychain" ]; then
  keychain=$(security default-keychain -d user 2>/dev/null | sed -e 's/^ *"//' -e 's/" *$//')
  [ -n "$keychain" ] || fail "this Mac has no default keychain — name one with --keychain"
fi
[ -f "$keychain" ] || fail "no keychain at $keychain"

# The SHA-1 of the one certificate of that name in the keychain, lower-case.
leaf_in_keychain() {
  security find-certificate -a -c "$name" -Z "$keychain" 2>/dev/null | awk '/^SHA-1 hash:/{print tolower($NF)}'
}
have=$(leaf_in_keychain)

if [ -n "$leaf_only" ]; then
  [ -n "$have" ] || fail "no certificate named '$name' in $keychain"
  [ "$(printf '%s\n' "$have" | wc -l | tr -d ' ')" = 1 ] || fail "more than one certificate matches '$name' in $keychain — name it exactly"
  printf '%s\n' "$have"
  exit 0
fi

[ -z "$have" ] || fail "$keychain already holds a certificate named '$name' — nothing made. The certificate IS the release identity: this script never replaces one. (Its SHA-1: tools/make-release-identity.sh '$name' --leaf)"

slug=$(printf '%s' "$name" | tr ' ' '-')
[ -n "$out" ] || out=$HOME/.shroom/release-identity
for f in p12 p12.base64 password cer sha1; do
  [ ! -e "$out/$slug.$f" ] || fail "$out/$slug.$f is already there — nothing made. Move the old files away first if you mean to start again."
done
# Secrets stay out of git: asked of the nearest folder that exists, before anything is made.
near=$out
while [ ! -d "$near" ]; do near=$(dirname "$near"); done
if command -v git >/dev/null 2>&1 && git -C "$near" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  fail "$out is inside a git checkout — secrets stay out of git. Choose another --out."
fi
umask 077
mkdir -p "$out"
chmod 700 "$out"
out=$(cd "$out" && pwd)

work=$(mktemp -d "${TMPDIR:-/tmp}/shroom-identity.XXXXXX")
cleanup() { rm -rf "$work"; }
trap cleanup EXIT

# A leaf certificate that signs code and nothing else.
cat >"$work/req.cnf" <<EOF
[ req ]
distinguished_name = dn
x509_extensions = ext
prompt = no
[ dn ]
CN = $name
[ ext ]
basicConstraints = critical, CA:false
keyUsage = critical, digitalSignature
extendedKeyUsage = critical, codeSigning
subjectKeyIdentifier = hash
EOF
"$OPENSSL" req -x509 -newkey rsa:3072 -nodes -sha256 -days "$days" -config "$work/req.cnf" \
  -keyout "$work/key.pem" -out "$work/cert.pem" >"$work/openssl.log" 2>&1 || { cat "$work/openssl.log" >&2; fail "couldn't make the certificate"; }

# The .p12's password: 40 random characters, written to its file and read from there.
printf '%s' "$("$OPENSSL" rand -base64 30 | tr -d '\n')" >"$work/password"
[ "$(wc -c <"$work/password" | tr -d ' ')" -ge 32 ] || fail "couldn't make a password"
# The older PKCS#12 ciphers on purpose: they are the ones `security import` reads on every
# macOS this ships for (a modern OpenSSL's defaults fail it with "MAC verification failed").
p12() { # <out.p12> <password-file>
  "$OPENSSL" pkcs12 -export -inkey "$work/key.pem" -in "$work/cert.pem" -name "$name" \
    -keypbe PBE-SHA1-3DES -certpbe PBE-SHA1-3DES -macalg sha1 -out "$1" -passout "file:$2" >>"$work/openssl.log" 2>&1 \
    || { cat "$work/openssl.log" >&2; fail "couldn't write the .p12"; }
}
p12 "$work/release.p12" "$work/password"
sha1=$("$OPENSSL" x509 -in "$work/cert.pem" -noout -fingerprint -sha1 | sed -e 's/^.*=//' -e 's/://g' | tr 'A-F' 'a-f')
[ ${#sha1} -eq 40 ] || fail "couldn't read the certificate's SHA-1"
# The .p12 and its password open each other, and hold this certificate — proven before
# either is kept.
back=$("$OPENSSL" pkcs12 -in "$work/release.p12" -passin "file:$work/password" -nokeys 2>/dev/null | "$OPENSSL" x509 -noout -fingerprint -sha1 | sed -e 's/^.*=//' -e 's/://g' | tr 'A-F' 'a-f')
[ "$back" = "$sha1" ] || fail "the .p12 doesn't open with its password — nothing kept"

# Into the keychain, by a copy under a one-time password (see the header).
printf '%s' "$("$OPENSSL" rand -hex 24)" >"$work/once"
p12 "$work/once.p12" "$work/once"
security import "$work/once.p12" -k "$keychain" -f pkcs12 -P "$(cat "$work/once")" -T /usr/bin/codesign >"$work/import.log" 2>&1 \
  || { cat "$work/import.log" >&2; fail "couldn't put the identity into $keychain — nothing kept"; }
[ "$(leaf_in_keychain)" = "$sha1" ] || fail "the keychain doesn't hold the new certificate — nothing kept"
security find-identity -p codesigning "$keychain" 2>/dev/null | grep -q "\"$name\"" || fail "the keychain holds the certificate but not its key — nothing kept"

cp "$work/release.p12" "$out/$slug.p12"
base64 -i "$work/release.p12" | tr -d '\n' >"$out/$slug.p12.base64"
cp "$work/password" "$out/$slug.password"
"$OPENSSL" x509 -in "$work/cert.pem" -outform DER -out "$out/$slug.cer"
printf '%s\n' "$sha1" >"$out/$slug.sha1"
chmod 600 "$out/$slug.p12" "$out/$slug.p12.base64" "$out/$slug.password"
chmod 644 "$out/$slug.cer" "$out/$slug.sha1"
until=$("$OPENSSL" x509 -in "$work/cert.pem" -noout -enddate | sed 's/^notAfter=//')

cat <<EOF
Made the release identity "$name".

  certificate SHA-1   $sha1
  valid until         $until
  in the keychain     $keychain
  files               $out

    $slug.p12          the certificate and its key, password-protected
    $slug.p12.base64   the same as one line of text
    $slug.password     the .p12's password
    $slug.cer          the certificate alone (public)
    $slug.sha1         its SHA-1 (public)

Next — docs/RELEASING.md walks through each of these:

1. Give GitHub the two secrets, in the environment named "release" (the values are read
   from the files; nothing shows on screen):

     gh secret set RELEASE_CERT_P12_BASE64 --repo bagrat/shroom --env release < "$out/$slug.p12.base64"
     gh secret set RELEASE_CERT_PASSWORD --repo bagrat/shroom --env release < "$out/$slug.password"

2. Pin the certificate in the repo, so no release is ever signed with another one:

     cp "$out/$slug.sha1" tools/release-identity.sha1

3. Back it up, offline: copy $slug.p12 and $slug.password to two places that aren't
   this Mac and aren't GitHub (GitHub never shows a secret again), and $slug.cer with
   them (public; signing a release by hand on this Mac asks for it). Whoever has the first
   two can sign as shroom; without them, a lost keychain means every release after it asks
   every user for Screen Recording again.

4. Then remove $out from this Mac.
EOF

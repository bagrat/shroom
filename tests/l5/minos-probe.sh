#!/bin/sh
# The macOS floor's probe (DECISIONS B6, B83): every Apple API a built shroom.app can reach,
# checked against the SDK headers' availability, so the floor is a measured fact.
#
# What it reads, from each executable in Contents/MacOS (the app and its encoder):
#   - the symbols it links (`nm -um`): C functions, constants, Objective-C classes;
#   - every identifier-shaped string in it: the class names and selectors objc2 and cidre
#     look up at run time (`objc_getClass` / `NSClassFromString` / `sel_registerName`
#     take strings, so the linker never sees them).
# What it checks them against: clang's own reading of the SDK headers for every framework
# the binaries link (`-ast-dump`: each class, protocol, method, property, function and
# constant with its `API_AVAILABLE(macos(…))`, a member inheriting its container's). A name
# declared in more than one place counts at its oldest declaration — a selector shared by an
# old class and a new one is not, by itself, a reason to raise the floor.
#
# It prints every name newer than 12.3 (ScreenCaptureKit's own floor) with the version it needs, and fails when one is newer
# than the bundle's floor (LSMinimumSystemVersion) unless it is on the list below of names
# known to be guarded or never called — each with where that was read.
#
# Usage: tests/l5/minos-probe.sh <path/to/shroom.app> [report.tsv]
set -eu
app=$1
report=${2:-}
floor=${SHROOM_PROBE_FLOOR:-$(plutil -extract LSMinimumSystemVersion raw -o - "$app/Contents/Info.plist")}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

: >"$work/undef.txt"
: >"$work/strings.txt"
: >"$work/frameworks.txt"
for bin in "$app/Contents/MacOS/"*; do
  nm -um "$bin" >>"$work/undef.txt"
  # Names looked up at run time matter only in a binary that speaks Objective-C; the encoder
  # (plain C) is read by its linked symbols — its strings are option names, and the few
  # symbols it looks up itself (dlsym) it guards; it is compiled for the floor with clang's
  # availability warnings on (tools/build-ffmpeg.sh, B82).
  if otool -L "$bin" | grep -q '/usr/lib/libobjc'; then
    strings -a -n 3 "$bin" | grep -E '^[A-Za-z_][A-Za-z0-9_:]*$' >>"$work/strings.txt" || true
  fi
  otool -L "$bin" | sed -n 's#.*/System/Library/Frameworks/\([A-Za-z0-9]*\)\.framework/.*#\1#p' >>"$work/frameworks.txt"
done
sort -u -o "$work/strings.txt" "$work/strings.txt"
for fw in $(sort -u "$work/frameworks.txt"); do
  case "$fw" in
    IOKit) echo '#import <IOKit/IOKitLib.h>' ;;
    *) echo "#import <$fw/$fw.h>" ;;
  esac
done >"$work/all.m"
clang -x objective-c -fsyntax-only -mmacosx-version-min=11.0 -Xclang -ast-dump "$work/all.m" 2>"$work/clang.err" >"$work/ast.txt" \
  || { cat "$work/clang.err"; echo "clang couldn't read the SDK headers"; exit 1; }

python3 - "$work" "$floor" "$report" <<'PY'
import collections, re, sys
work, floor_s, report = sys.argv[1], sys.argv[2], sys.argv[3]
def ver(s):
    return tuple(int(x) for x in s.split('.')) if s else (0,)
def show(v):
    return '.'.join(map(str, v))
floor = ver(floor_s)

# Names newer than the floor that the shipped code never reaches unguarded. Each was read in
# the source of the revision Cargo.lock pins; the probe fails if a new one appears.
KNOWN = {
    # cidre (rev bf84b670) looks every class it binds up by name at load —
    # `NSClassFromString(@"…")` in pomace/*/*.h, nil on an older macOS — and nothing in
    # shroom, scap-screencapturekit or cap-camera-avfoundation (rev fb3f2438) uses these.
    'MTL4RenderPassDescriptor': 'cidre class lookup, never used', 'MTL4PipelineOptions': 'cidre class lookup, never used',
    'MTL4MachineLearningPipelineDescriptor': 'cidre class lookup, never used', 'MTL4CounterHeapDescriptor': 'cidre class lookup, never used',
    'MTL4CommitOptions': 'cidre class lookup, never used', 'MTL4CommandBufferOptions': 'cidre class lookup, never used',
    'MTL4CommandAllocatorDescriptor': 'cidre class lookup, never used', 'MTL4ArgumentTableDescriptor': 'cidre class lookup, never used',
    'MTLResidencySetDescriptor': 'cidre class lookup, never used', 'VNCalculateImageAestheticsScoresRequest': 'cidre class lookup, never used',
    'SCRecordingOutput': 'cidre class lookup, never used', 'SCRecordingOutputConfiguration': 'cidre class lookup, never used',
    'AVCaptureSlider': 'cidre class lookup, never used', 'AVCaptureSystemZoomSlider': 'cidre class lookup, never used',
    'AVCaptureSystemExposureBiasSlider': 'cidre class lookup, never used', 'AVCaptureIndexPicker': 'cidre class lookup, never used',
    'VNGenerateForegroundInstanceMaskRequest': 'cidre class lookup, never used', 'SCScreenshotManager': 'cidre class lookup, never used',
    'SCContentSharingPicker': 'cidre class lookup, never used', 'SCContentSharingPickerConfiguration': 'cidre class lookup, never used',
    'AVSampleBufferVideoRenderer': 'cidre class lookup, never used', 'AVExternalStorageDevice': 'cidre class lookup, never used',
    'AVCaptureDeviceRotationCoordinator': 'cidre class lookup, never used', 'AVAudioApplication': 'cidre class lookup, never used',
    # cap-camera-avfoundation's device list asks for these only behind macos_available("14.0").
    'AVCaptureDeviceTypeExternal': 'cap-camera-avfoundation camera list, behind macos_available("14.0")',
    'AVCaptureDeviceTypeContinuityCamera': 'cap-camera-avfoundation camera list, behind macos_available("14.0")',
    # wry 0.55 (wkwebview/mod.rs) uses a custom data store only when one is configured and
    # the OS says it can (its own operatingSystemVersion check); tauri configures none.
    'dataStoreForIdentifier:': 'wry, only with a data store id, behind its OS check',
    'fetchAllDataStoreIdentifiers:': 'wry, only with a data store id, behind its OS check',
    'removeDataStoreForIdentifier:completionHandler:': 'wry, only with a data store id, behind its OS check',
}

idx = collections.defaultdict(list)
stack = []  # [depth, name, availability]
pending = None  # [depth, kind, name, container, own, container_avail]
ansi = re.compile(r'\x1b\[[0-9;]*m')
node = re.compile(r'^([ |`]*[|`]-)(\w+)(.*)$')
OURS = ('ObjCInterfaceDecl', 'ObjCProtocolDecl', 'ObjCCategoryDecl', 'ObjCMethodDecl', 'ObjCPropertyDecl', 'FunctionDecl', 'VarDecl', 'EnumConstantDecl')
def flush():
    global pending
    if pending:
        _, kind, name, cont, own, cav = pending
        if kind in ('ObjCInterfaceDecl', 'ObjCProtocolDecl'):
            if own is not None:
                idx[('class', name)].append((cont, ver(own)))
        elif kind == 'ObjCMethodDecl':
            idx[('sel', name)].append((cont, ver(own or cav)))
        elif kind == 'ObjCPropertyDecl':
            v = ver(own or cav)
            idx[('sel', name)].append((cont, v))
            idx[('sel', 'set' + name[:1].upper() + name[1:] + ':')].append((cont, v))
        elif kind != 'ObjCCategoryDecl':
            idx[('sym', name)].append((cont, ver(own)))
    pending = None
with open(f'{work}/ast.txt', errors='replace') as ast:
    for raw in ast:
        m = node.match(ansi.sub('', raw.rstrip('\n')))
        if not m:
            continue
        depth, kind, rest = len(m.group(1)), m.group(2), m.group(3)
        # A node at or above the pending declaration's level ends it; above a container's, ends it.
        if kind != 'AvailabilityAttr' or (pending and depth <= pending[0]):
            if pending and depth <= pending[0]:
                flush()
        while stack and stack[-1][0] >= depth:
            stack.pop()
        if kind == 'AvailabilityAttr':
            a = re.search(r' macos (\d+(?:\.\d+)*)', rest)
            if not a:
                continue
            # Only a declaration's own attribute (its direct child) counts.
            if pending and depth == pending[0] + 2:
                pending[4] = a.group(1)
                if pending[1] in ('ObjCInterfaceDecl', 'ObjCProtocolDecl', 'ObjCCategoryDecl') and stack and stack[-1][0] == pending[0]:
                    stack[-1][2] = a.group(1)
            continue
        if kind not in OURS:
            continue
        flush()
        if kind in ('ObjCInterfaceDecl', 'ObjCProtocolDecl', 'ObjCCategoryDecl'):
            names = re.findall(r'(?:> line:\d+:\d+|> col:\d+|>)\s+(?:implicit\s+)?(?:referenced\s+)?(\w+)', rest)
            name = names[-1] if names else '?'
            stack.append([depth, name, None])
            pending = [depth, kind, name, '', None, None]
            continue
        if kind == 'ObjCMethodDecl':
            mm = re.search(r' [-+] (\S+) \'', rest)
        elif kind == 'ObjCPropertyDecl':
            mm = re.search(r' (\w+) \'', rest)
        else:
            mm = re.search(r' (?:used |referenced |implicit )*(\w+) \'', rest)
        name = mm.group(1) if mm else '?'
        objc = kind.startswith('ObjC')
        pending = [depth, kind, name, stack[-1][1] if stack and objc else '', None, stack[-1][2] if stack and objc else None]
    flush()

names = set()
for l in open(f'{work}/undef.txt'):
    m = re.search(r'external (\S+)', l)
    if m:
        s = m.group(1)
        if s.startswith('_OBJC_CLASS_$_'):
            names.add(s[len('_OBJC_CLASS_$_'):])
        elif not s.startswith('_OBJC_METACLASS_$_'):
            names.add(s[1:])
names |= {l.strip() for l in open(f'{work}/strings.txt')}

rows = []
for n in sorted(names):
    for kind in ('class', 'sel', 'sym'):
        d = idx.get((kind, n))
        if d:
            need = min(v for _, v in d)
            where = sorted({c for c, v in d if v == need and c}) or ['']
            rows.append((need, kind, n, where[0]))
rows.sort(key=lambda r: (r[0], r[2]), reverse=True)
bad = []
lines = []
for need, kind, n, where in rows:
    if need <= (12, 3):
        continue
    why = KNOWN.get(n, '')
    if need > floor and not why:
        bad.append(n)
    lines.append(f'{show(need)}\t{kind}\t{n}\t{where}\t{why or "reached unguarded — sets the floor" if need > floor else why or "within the floor"}')
print(f'{len(rows)} Apple names found in the bundle; {len(lines)} newer than 12.3, ScreenCaptureKit’s own (floor: {floor_s}):')
for l in lines:
    print('  ' + l)
top = max((r[0] for r in rows if r[2] not in KNOWN), default=(0,))
print(f'newest name reached without a guard: macOS {show(top)}')
if report:
    open(report, 'w').write('needs\tkind\tname\tdeclared on\twhy it is safe\n' + '\n'.join(lines) + '\n')
if bad:
    print('FAIL newer than the floor and not known to be guarded: ' + ', '.join(bad))
    sys.exit(1)
if top > floor:
    print(f'FAIL the code reaches macOS {show(top)} API, above the floor {floor_s}')
    sys.exit(1)
print(f'ok   nothing the bundle reaches needs more than macOS {floor_s}')
PY

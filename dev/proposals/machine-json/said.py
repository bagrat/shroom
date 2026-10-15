import json,re,glob,html,difflib,collections,sys,os
import pathlib; R=str(pathlib.Path(__file__).resolve().parents[3])+'/'
OUT=sys.argv[1]
files=[]
for pat in ['crates/shroom-core/src/**/*.rs','app/src/**/*.svelte','app/src/**/*.ts','app/src-tauri/src/**/*.rs','app/src-tauri/Info.plist','worker/src/*.js','site/worker.js','site/index.html','installer/install.sh','app/src-tauri/tauri.conf.json']:
    files+=glob.glob(R+pat,recursive=True)
def surf(f):
    f=f[len(R):]
    for k,v in [('crates/shroom-core/src/copy.rs','core-copy'),('crates/','core'),('app/src-tauri','tauri'),('app/src','webview'),('worker','worker'),('site','site'),('installer','installer')]:
        if f.startswith(k): return v
def norm(t):
    t=t.replace('\u2019',"'").replace('\u2018',"'").replace('\u201c','"').replace('\u201d','"').replace('\u00a0',' ').replace('\u2009',' ').replace('\u202f',' ')
    return re.sub(r'\s+',' ',t).strip()
corp={}
lits=[]  # (file, text)
for f in files:
    raw=open(f,encoding='utf-8',errors='replace').read()
    if f.endswith('.rs'):
        i=raw.find('#[cfg(test)]')
        if i>=0: raw=raw[:i]
        raw=re.sub(r'(?m)^\s*//.*$','',raw)
        raw=re.sub(r'(?m)\s//[ /!].*$','',raw)
    if f.endswith(('.js','.ts','.svelte','.html')):
        raw=re.sub(r'(?m)^\s*//.*$','',raw)
        raw=re.sub(r'/\*.*?\*/','',raw,flags=re.S)
        raw=re.sub(r'<!--.*?-->','',raw,flags=re.S)
    t=raw
    if f.endswith('.rs'):
        t=re.sub(r'\\\n\s*','',t)
        t=re.sub(r'\\u\{([0-9a-fA-F]+)\}',lambda m:chr(int(m.group(1),16)),t)
        t=t.replace('\\"','"').replace("\\'","'").replace('\\n','\n')
    if f.endswith(('.js','.ts','.svelte','.html')):
        t=re.sub(r'\\u([0-9a-fA-F]{4})',lambda m:chr(int(m.group(1),16)),t)
        t=re.sub(r'\\u\{([0-9a-fA-F]+)\}',lambda m:chr(int(m.group(1),16)),t)
        t=t.replace("\\'","'").replace('\\"','"').replace('\\`','`')
        t=html.unescape(t)
    stripped=re.sub(r'<[^<>]{0,400}?>','',t)
    corp[f]=(norm(t),norm(stripped))
    # literals
    if f.endswith('.rs'):
        for m in re.finditer(r'r#"(.*?)"#|"((?:[^"\\\n]|\\.)*)"',raw.replace('\\\n','')):
            s=m.group(1) or m.group(2) or ''
            lits.append((f,norm(s.replace('\\"','"'))))
    elif f.endswith(('.js','.ts','.svelte')):
        for m in re.finditer(r'"((?:[^"\\\n]|\\.)*)"|\'((?:[^\'\\\n]|\\.)*)\'|`((?:[^`\\]|\\.)*)`',raw):
            s=next(g for g in m.groups() if g is not None)
            lits.append((f,norm(html.unescape(s))))
        if f.endswith('.svelte') or f.endswith('.js'):
            for m in re.finditer(r'>([^<>{}]*[A-Za-z][^<>{}]*)<',raw):
                lits.append((f,norm(html.unescape(m.group(1)))))
    elif f.endswith('.html'):
        for m in re.finditer(r'>([^<>]*[A-Za-z][^<>]*)<',raw):
            lits.append((f,norm(html.unescape(m.group(1)))))
    elif f.endswith('.sh'):
        for line in raw.splitlines():
            for m in re.finditer(r'"([^"]*)"|\'([^\']*)\'',line):
                s=m.group(1) or m.group(2) or ''
                lits.append((f,norm(s)))
            if line.startswith('#'): lits.append((f,norm(line)))
    else:
        for m in re.finditer(r'<string>(.*?)</string>|"([^"]*)"',raw): lits.append((f,norm(m.group(1) or m.group(2) or '')))
lits=[(f,s) for f,s in lits if s]
HOLE=r"[a-z]+\.my(?:shroom|name)\.workers\.dev(?:/[a-z0-9/]+)?|v\.(?:myshroom|mine)\.workers\.dev(?:/[a-z0-9/]+)?|\b(?:[Tt]wo|three|six|five|four|seven)\b|(?:Aug|Sep|Jul) \d+|MacBook Air|Where the 502s come from|Bagrat's Account|\d[\d:.,%/]*\d|\d|Bagrat|Mher|Ani|Lusine|Aram|myshroom|b4kq9d|AbCdEfGhIjKlMnOp|\bN\b|\bM\b|Built-in Display|LG UltraFine|MacBook Pro Microphone|FaceTime HD Camera|Acme"
said=[]
for i in range(1,7):
    for s in json.load(open(R+f'history/spec/r{i}.json'))['states']:
        for l in s['said_lines']: said.append((i,s['id'],s['surface'],l['role'],l['copy_status'],l['text']))
def find(text):
    n=norm(text)
    for f,(a,b) in corp.items():
        if n in a or n in b: return 'verbatim',f,n
    frags=[norm(x) for x in re.split(HOLE,text)]
    frags=[x for x in frags if len(re.sub(r'\W','',x))>=2]
    if frags and len(frags)>=1 and re.search(HOLE,text):
        rx='.{0,160}?'.join(re.escape(x) for x in frags)
        for f,(a,b) in corp.items():
            for c in (a,b):
                m=re.search(rx,c,re.S)
                if m: return 'template',f,m.group(0)[:200]
    return None
res=[]
cand=[(f,s) for f,s in lits if len(s)>=3 and re.search('[A-Za-z]',s)]
for row in said:
    text=row[5]
    if '\n' in text and row[2]=='terminal':
        lines=[x for x in text.split('\n') if x.strip()]
        ok=sum(1 for x in lines if find(x))
        if ok==len(lines): res.append(row+('verbatim','installer/install.sh','(multi-line, all lines)')); continue
    r=find(text)
    if r: res.append(row+r); continue
    n=norm(text); best=(0,None,None)
    for f,s in cand:
        if abs(len(s)-len(n))>max(40,len(n)): continue
        q=difflib.SequenceMatcher(None,n.lower(),s.lower())
        if q.real_quick_ratio()<best[0] or q.quick_ratio()<best[0]: continue
        r2=q.ratio()
        if r2>best[0]: best=(r2,f,s)
    parts=[norm(x) for x in re.split(r'^[✓•→] |\s[—(]\s?|\)$|'+HOLE,text) if x and len(re.sub(r'\W','',x))>=3]
    if parts and all(any(any(x in c for c in v) for v in corp.values()) for x in parts):
        res.append(row+('composed','(parts)',' | '.join(parts))); continue
    if best[0]>=0.6: res.append(row+('reworded',best[1],best[2],round(best[0],2)))
    else: res.append(row+('missing',best[1],best[2],round(best[0],2)))
c=collections.Counter(r[6] for r in res)
print(c)
json.dump(res,open(OUT+'/said_lines.json','w'),indent=1,ensure_ascii=False)
# UI strings without said-line
saidn=[norm(r[5]).lower() for r in said]
prose=set()
for f,s in lits:
    sf=surf(f)
    if sf in ('core',): continue  # only copy.rs from core
    if '/tests/' in f or f.endswith('install.sh') and not s: continue
    if len(s)<4 or not re.search(r'[A-Za-z]{2,}',s) : continue
    if not (' ' in s and re.match(r"[A-Z“\"'✓→•$(]",s)) : continue
    if re.search(r'[{}<>;=]|\$\{|::|\(\)|\bfn\b|^\w+\(|https?://\S+$',s): continue
    prose.add((sf,s))
orph=[]
for sf,s in sorted(prose):
    sl=s.lower()
    if any(sl in x or (len(x)>6 and x in sl) for x in saidn): continue
    if max((difflib.SequenceMatcher(None,sl,x).ratio() for x in saidn if abs(len(x)-len(sl))<60),default=0)>=0.75: continue
    orph.append((sf,s))
print('prose UI strings',len(prose),'without said-line',len(orph), collections.Counter(x[0] for x in orph))
json.dump(orph,open(OUT+'/ui_strings_without_said_line.json','w'),indent=1,ensure_ascii=False)

# ── strict pass: right surface's files, and short labels must be a whole literal ──
def allowed(surface,f):
    f=f[len(R):]
    if surface=='web-page': return f.startswith(('worker/','site/'))
    if surface=='terminal': return f.startswith('installer/')
    return f.startswith(('crates/shroom-core/src/copy.rs','crates/shroom-core/src/derive/','app/'))
litset=collections.defaultdict(set)
for f,s in lits: litset[f].add(s)
def strict(row):
    surface,text=row[2],row[5]
    lines=[x for x in text.split('\n') if x.strip()] if '\n' in text else [text]
    ok=True
    for t in lines:
        n=norm(t); found=False
        for f,(a,b) in corp.items():
            if not allowed(surface,f): continue
            if len(n)<12:
                if n in litset[f] or ('>'+n+'<') in open(f,encoding='utf-8',errors='replace').read(): found=True;break
            elif n in a or n in b: found=True;break
            else:
                frags=[norm(x) for x in re.split(HOLE,t)]; frags=[x for x in frags if len(re.sub(r'\W','',x))>=2]
                if re.search(HOLE,t) and frags:
                    rx='.{0,160}?'.join(re.escape(x) for x in frags)
                    if re.search(rx,a,re.S) or re.search(rx,b,re.S): found=True;break
        ok=ok and found
    return ok
sc=collections.Counter(); sl=[]
for row,r in zip(said,res):
    k=strict(row); sc[(r[6],k)]+=1
    if r[6] in ('verbatim','template') and not k: sl.append(row)
print('strict:',sc)
json.dump(sl,open(OUT+'/strict_fail.json','w'),indent=1,ensure_ascii=False)

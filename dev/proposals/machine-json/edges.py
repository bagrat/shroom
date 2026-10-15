import json,re,sys,collections
import pathlib; R=str(pathlib.Path(__file__).resolve().parents[3])+'/'
m=json.load(open(R+'history/spec/map.json'))
fx={}
for i in range(1,7):
    for s in json.load(open(R+f'history/spec/r{i}.json'))['states']:
        fx.setdefault(s['id'],[]).append((i,s))
src=open(R+'crates/shroom-core/src/apply.rs').read()
arm=set(re.findall(r'\("([^"]+)", "([^"]+)"\)',src))
arm|={("member-welcome","joining"),("join-switch-confirm","joining"),("config","quit-confirm"),("idle-ready","quit-confirm"),("paused","countdown"),("discard-confirm","countdown")}
sup=open(R+'crates/shroom-core/tests/common/support.rs').read()
# parse map_names
mn={}
for line in sup[sup.index('fn map_names'):sup.index('other => return')].splitlines():
    mm=re.match(r'\s*(.*?)\s*=>\s*&\[(.*)\],',line)
    if mm:
        keys=re.findall(r'"([^"]+)"',mm.group(1)); vals=re.findall(r'"([^"]+)"',mm.group(2))
        for k in keys: mn[k]=vals
names=lambda n: mn.get(n,[n])
extra=re.findall(r'\("([^"]+)", "([^"]+)", "',sup[sup.index('EXTRA_ARM_EDGES'):sup.index('pub fn edge_listed')])
extra=set(extra)
medges={(e['from'],e['to']):e for e in m['edges']}
def onmap(a,b): return [(f,t) for f in names(a) for t in names(b) if (f,t) in medges]
def infix(a,b):
    for i,s in fx.get(a,[]):
        for tr in s['transitions']:
            if tr['to']==b or (tr['to'].startswith(b) and tr['to'][len(b):len(b)+1] in (' ','(')): return True
    return False
out=collections.defaultdict(list)
mapped_by_arm=collections.defaultdict(list)
for a,b in sorted(arm):
    om=onmap(a,b)
    lit=(a,b) in medges
    for e in om: mapped_by_arm[e].append((a,b))
    cat='map-literal' if lit else ('map-via-alias' if om else ('fixture-only' if infix(a,b) else ('EXTRA' if (a,b) in extra else 'UNLISTED')))
    out[cat].append((a,b,om))
print('ARM EDGES',len(arm))
for k,v in out.items(): print(' ',k,len(v))
json.dump({k:[list(x[:2])+[x[2]] for x in v] for k,v in out.items()},open(sys.argv[1]+'/arm_edges.json','w'),indent=1)
# map edges not named by an arm
# gather test lists
T={}
for f in ['onboarding','edges','region5','region6']:
    T[f]=open(R+f'crates/shroom-core/tests/{f}.rs').read()
def block(text,start,end_pat='\n];'):
    i=text.index(start); j=text.index(end_pat,i); return text[i:j]
def realized(text):
    i=text.index('fn realized()'); j=text.index('\n}\n',i); return set(re.findall(r'^\s*\("([^"]+)", "([^"]+)",',text[i:j],re.M))
real=set()
for f,t in T.items(): real|=realized(t)
lists={
 'UNNAMED(derived, edges.rs)':set(re.findall(r'\("([^"]+)", "([^"]+)", "',block(T['edges'],'const UNNAMED'))),
 'WORKER_EDGES':set(re.findall(r'\("([^"]+)", "([^"]+)", "',block(T['edges'],'const WORKER_EDGES'))),
 'OUT_OF_SCOPE(onboarding.rs)':set(re.findall(r'\("([^"]+)", "([^"]+)", "',block(T['onboarding'],'const OUT_OF_SCOPE'))),
 'NOT_REALIZED_HERE(region5)':set(re.findall(r'\("([^"]+)", "([^"]+)", "',block(T['region5'],'const NOT_REALIZED_HERE'))),
 'NOT_REALIZED_HERE(region6)':set(re.findall(r'\("([^"]+)", "([^"]+)", "',block(T['region6'],'const NOT_REALIZED_HERE'))),
}
res=collections.defaultdict(list)
for e,ed in medges.items():
    if e in mapped_by_arm: res['named-by-arm'].append(e); continue
    tags=[k for k,v in lists.items() if e in v]
    if e in real and not tags: tags=['realized-in-tests-unnamed']
    elif e in real: tags=['realized+'+tags[0]]
    res[tags[0] if tags else 'NOWHERE'].append(e)
print('MAP EDGES',len(medges))
for k,v in res.items(): print(' ',k,len(v))
json.dump({k:[list(e)+[medges[e].get('label'),medges[e].get('region')] for e in v] for k,v in res.items()},open(sys.argv[1]+'/map_edges.json','w'),indent=1)
# fixture transitions
ft=[]
for fid,ss in fx.items():
    for i,s in ss:
        for tr in s['transitions']: ft.append((fid,tr['to'],tr.get('trigger',''),i,s['surface']))
print('FIXTURE TRANSITIONS',len(ft))
def armhit(fid,to):
    for a,b in arm:
        if a==fid and (to==b or (to.startswith(b) and to[len(b):len(b)+1] in (' ','('))): return True
    return False
def maphit(fid,to):
    for f in names(fid):
        for t in names(to.split(' (')[0]):
            if (f,t) in medges: return True
    return False
fc=collections.Counter(); fl=collections.defaultdict(list)
for fid,to,trg,i,surf in ft:
    c='arm' if armhit(fid,to) else ('map-only' if maphit(fid,to) else 'neither')
    fc[c]+=1; fl[c].append((fid,to,trg,i,surf))
print(fc)
json.dump(fl,open(sys.argv[1]+'/fixture_transitions.json','w'),indent=1)

#!/usr/bin/env bash
# M6 security gate: static rules + dependency tools (CI strict; local missing => unverified).
set -euo pipefail
cd "$(dirname "$0")/.."
OUT="${1:-target/evidence/m6/security.json}"; mkdir -p "$(dirname "$OUT")"
python3 - "$OUT" <<'PY'
import json,subprocess,sys,pathlib,re
root=pathlib.Path('.'); findings=[]
def add(rule,severity,file,line,status,owner="security"):
    findings.append({"rule":rule,"severity":severity,"file":file,"line":line,"status":status,"owner":owner})
# secret patterns in tracked product code
for path in list(root.glob('apps/**/*.rs'))+list(root.glob('packages/**/*.rs'))+list(root.glob('crates/**/*.rs'))+list(root.glob('tools/**/*.rs')):
    if 'target' in path.parts: continue
    try: lines=path.read_text(errors='ignore').splitlines()
    except Exception: continue
    for i,line in enumerate(lines,1):
        if re.search(r'BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY|AKIA[0-9A-Z]{16}|sk-[A-Za-z0-9]{20,}',line):
            add('secret-literal','high',str(path),i,'fail')
        if re.search(r'\bunsafe\s*\{',line):
            add('unsafe-code','high',str(path),i,'fail')
        if re.search(r'\.(unwrap|expect)\s*\(',line) and '/tests/' not in str(path) and '#[cfg(test)]' not in line:
            add('unwrap-expect','medium',str(path),i,'review')
strict=subprocess.run(['bash','-lc','test "${ZEOKEEL_SECURITY_STRICT:-0}" = 1'],capture_output=True).returncode==0
for tool,args,probe in [('cargo',['deny','check'],['deny','--version']),('cargo',['audit'],['audit','--version']),('pnpm',['audit','--prod'],['audit','--version'])]:
    available=subprocess.run([tool]+probe,capture_output=True).returncode==0
    if not available:
        add('dependency-audit:'+tool+':'+args[0],'high' if strict else 'unverified','-',0,'fail' if strict else 'unverified'); continue
    result=subprocess.run([tool]+args,capture_output=True,text=True)
    if result.returncode == 0:
        add('dependency-audit:'+tool+':'+args[0],'info','-',0,'pass')
    else:
        network = 'audit endpoint' in (result.stderr+result.stdout) or 'network' in (result.stderr+result.stdout).lower()
        if network and not strict: add('dependency-audit:'+tool+':'+args[0],'unverified','-',0,'unverified')
        else: add('dependency-audit:'+tool+':'+args[0],'high','-',0,'fail')
high=[f for f in findings if f['severity']=='high' and f['status']=='fail']
result={"schema":1,"checks":len(findings),"high_fail":len(high),"findings":findings,"status":"pass" if not high else "fail"}
pathlib.Path(sys.argv[1]).write_text(json.dumps(result,indent=2)+"\n")
print(f"security audit: {len(findings)} findings, high_fail={len(high)}")
sys.exit(1 if high else 0)
PY

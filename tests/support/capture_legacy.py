"""Explicit offline refresh, never downloads or changes the supplied legacy checkout."""
import hashlib, importlib, json, pathlib, subprocess, sys, tempfile
legacy=pathlib.Path(sys.argv[1]).resolve()
revision='13b52b054c31f614dc272b18c195b8fa929aa595'
assert subprocess.check_output(['git','rev-parse','HEAD'],cwd=legacy,text=True).strip()==revision
assert not subprocess.check_output(['git','status','--porcelain','--untracked-files=no'],cwd=legacy,text=True)
sys.path.insert(0,str(legacy/'scripts')); old=importlib.import_module('flowguard_lib.stage_docs')
cases=[]
for state in old.VALID_STATUSES:
 text=f'| 阶段 | 02-architecture |\n| 阶段状态 | {state} |\n| 前置指纹 | project |\n| 验收指纹 | - |\n\nActual content\n'
 text=text.replace('| 验收指纹 | - |',f'| 验收指纹 | {old._content_hash(text)} |')
 cases.append((state,text,None))
cases.extend([
 ('body-invalidated',cases[3][1]+'changed content\n',None),
 ('unknown-state','| 阶段 | 02-architecture |\n| 阶段状态 | approved |\n',None),
 ('wrong-stage','| 阶段 | 99-unknown |\n| 阶段状态 | pending |\n',None),
 ('duplicate-state','| 阶段 | 02-architecture |\n| 阶段状态 | pending |\n| 阶段状态 | in_progress |\n',None),
 ('fenced-example','```md\n| 阶段 | 02-architecture |\n| 阶段状态 | pending |\n```\n',None),
 ('missing',None,None),
 ('directory',None,'directory'),
 ('invalid-utf8',None,'invalid-utf8'),
])
records=[]
for name,text,fault in cases:
 with tempfile.TemporaryDirectory() as directory:
  path=pathlib.Path(directory)/'docs/project/02-architecture.md';path.parent.mkdir(parents=True)
  if text is not None:path.write_text(text)
  elif fault=='directory':path.mkdir()
  elif fault=='invalid-utf8':path.write_bytes(b'\xff')
  try:
   result=old.read(directory,'feature-a','02-architecture'); result={k:v for k,v in result.items() if k in ('status','missing')}
  except Exception as error:result={'error':type(error).__name__}
  records.append(dict(name=name,text=text,fault=fault,legacy=result))
out={'revision':revision,'sourceSha256':hashlib.sha256((legacy/'scripts/flowguard_lib/stage_docs.py').read_bytes()).hexdigest(),'cases':records,'legal':old.LEGAL}
path=pathlib.Path(__file__).resolve().parents[2]/'fixtures/legacy/differential.json';path.write_text(json.dumps(out,ensure_ascii=False,indent=2)+'\n')
print(f'{len(records)} actual legacy read cases; {len(old.LEGAL)**2} transition pairs frozen; source {out["sourceSha256"]}')

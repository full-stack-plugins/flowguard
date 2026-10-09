#!/usr/bin/env python3
"""Fresh controlled joint fixture. No production authentication or write executor."""
from pathlib import Path
import copy, hashlib,json,os,shutil,subprocess,sys
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
DEST=Path(sys.argv[1]).resolve();DEST.mkdir(parents=True,exist_ok=False)
REPO=DEST/'repository';shutil.copytree(HERE.parent/'approval-bridge/source',REPO)
FG=Path(os.environ.get('FLOWGUARD_JOINT_GENERATOR',ROOT/'target/debug/examples/generate_joint'))
TG=Path('/workspace/guard-implementation-ledger/fixed-tools/flowguard-joint-testguard-fixture')
CONSUMER=Path('/workspace/guard-implementation-ledger/fixed-tools/flowguard-independent-56a2c95/flowguard')
OUT=DEST
NATIVE=Path('/workspace/guard-implementation-ledger/codeguard-ruff-native-capture/codeguard-f5661d5')
CG=Path('/workspace/guard-implementation-ledger/codeguard-ruff-cli-capture/codeguard')
RUFF=Path('/workspace/guard-toolchain/python-native/bin/ruff')
FIXED={str(NATIVE):'b7576afa78f5ab6fdc8133650a1c75dc113357ac927e553b4f5c67731eea3a8c',str(CG):'df8fe1a6e208db0bdc8f7fe6b443dbc826b7ce377a0568989ce5e37699940dba',str(RUFF):'f3eb080f0173e1a5884fabcc2c836f5a0ebea35fccf252e718fc71b17f821ef7'}
sha=lambda b:hashlib.sha256(b).hexdigest()
def source_observation(path):
 # An archive has no Git identity; never report an enclosing workspace's HEAD.
 top=subprocess.run(['git','-C',str(path),'rev-parse','--show-toplevel'],capture_output=True,text=True)
 if top.returncode or Path(top.stdout.strip()).resolve()!=path.resolve():return {'sha':None,'dirty':None,'profile':'archive; source pins supplied by independent review'}
 return {'sha':subprocess.check_output(['git','-C',str(path),'rev-parse','HEAD']).decode().strip(),'dirty':subprocess.check_output(['git','-C',str(path),'status','--porcelain']).decode(),'profile':'runtime observation, not build provenance'}

for path,digest in FIXED.items():assert sha(Path(path).read_bytes())==digest,path
ENV={k:os.environ[k] for k in ['PATH','HOME','CARGO_HOME','RUSTUP_HOME','FLOWGUARD_JOINT_INVALIDATION'] if k in os.environ};ENV.update(CARGO_INCREMENTAL='0',CARGO_BUILD_JOBS='1',CARGO_TARGET_DIR='/workspace/guard-implementation/flowguard/target',GIT_OPTIONAL_LOCKS='0',GIT_CONFIG_NOSYSTEM='1',GIT_CONFIG_GLOBAL='/dev/null',GIT_AUTHOR_NAME='Fixture',GIT_AUTHOR_EMAIL='fixture@example.invalid',GIT_COMMITTER_NAME='Fixture',GIT_COMMITTER_EMAIL='fixture@example.invalid',GIT_AUTHOR_DATE='2026-10-09T00:00:00Z',GIT_COMMITTER_DATE='2026-10-09T00:00:00Z')
records=[]
def run(label,argv,expected=0):
 p=subprocess.run([str(x) for x in argv],cwd=REPO,env=ENV,capture_output=True,timeout=60)
 records.append({'label':label,'argv':[str(x) for x in argv],'cwd':str(REPO),'exit':p.returncode,'stdoutSha256':sha(p.stdout),'stderrSha256':sha(p.stderr)})
 assert p.returncode==expected,(label,p.returncode,p.stderr.decode(errors='replace'))
 return p

FIXED[str(TG)]='d9547e4ceba0770c16d64666f1ef9294bb5900932eebe06d38fd4e5d921b4a16'
FIXED[str(CONSUMER)]='9d0dadea6bcafce083a487548ffb4b082945bfa136f693760e7403de2b3fbf24'
for path,digest in FIXED.items():assert sha(Path(path).read_bytes())==digest,path
ENV['GIT_CONFIG_GLOBAL']='/dev/null'
def write(path,value):path.write_text(json.dumps(value,separators=(',',':')))
def read(path):return json.loads(path.read_bytes())
def git(*args):return run('git-'+args[0],['/usr/bin/git',*args]).stdout.decode().strip()
def state():return {str(p.relative_to(REPO)):sha(p.read_bytes()) for p in REPO.rglob('*') if p.is_file()}
# Shared project baseline is distinct from per-scope SpecGuard approval records.
(REPO/'baseline.json').write_text('{"version":"joint.project/v1","project":"shared-02","revision":1}\n')
for name in ['a','b']:
 (REPO/'specs'/f'{name}.md').write_text(f'---\nformat: markdown-explicit/v1\nnamespace: demo\n---\n## Requirement: {name.upper()}\nThe fixture returns the original answer.\n### Acceptance: AC-{name.upper()}\nThe trusted local test observes 42.\n')
run('cargo-lock',['cargo','generate-lockfile','--offline','--manifest-path',REPO/'Cargo.toml'])
git('-c','init.defaultBranch=main','init','-q');git('add','.');git('commit','-qm','joint source baseline');BASE=git('rev-parse','HEAD')
profiles={}
for name,requirements in [('A',['demo:A']),('B',['demo:B']),('M1',['demo:A','demo:B']),('M2',['demo:A','demo:B'])]:
 OUT=DEST/name;OUT.mkdir()
 profile={'task':('joint-queue' if name.startswith('M') else 'joint-'+name),'requirements':requirements,'base':BASE,'group':None,'members':[],'approved_baseline':str(OUT/'specguard/baseline.json')}
 profile_path=OUT/'baseline-profile.json';write(profile_path,profile);ENV['FLOWGUARD_JOINT_PROFILE']=str(profile_path)
 run('baseline-'+name,[FG,'sg-baseline',REPO,OUT]);profiles[name]=profile
for name in ['A','B']:
 git('checkout','--detach',BASE)
 file=REPO/'specs'/f'{name.lower()}.md';file.write_text(file.read_text().replace('original answer','reviewed answer'))
 git('commit','-qam','requirement '+name);profiles[name]['candidate']=git('rev-parse','HEAD')
A=profiles['A']['candidate'];B=profiles['B']['candidate']
tree=git('merge-tree','--write-tree',A,B);M1=git('commit-tree',tree,'-p',A,'-p',B,'-m','M1')
git('checkout','--detach',BASE)
file=REPO/'core/src/lib.rs';file.write_text(file.read_text()+'\n// advanced common base\n');git('commit','-qam','base advance');BASE2=git('rev-parse','HEAD')
tree=git('merge-tree','--write-tree',BASE2,M1);M2=git('commit-tree',tree,'-p',BASE2,'-p',M1,'-m','M2')
profiles['M1'].update(candidate=M1,group='M1',members=[A,B])
profiles['M2'].update(candidate=M2,base=BASE2,group='M2',members=[B,A])
project_digest=sha((REPO/'baseline.json').read_bytes())
provider_hashes={}
for name,profile in profiles.items():
 OUT=DEST/name;case_name=name;OID=profile['candidate'];git('checkout','--detach',OID)
 frozen_profile={k:v for k,v in profile.items() if k!='candidate'}
 write(OUT/'profile.json',frozen_profile);ENV['FLOWGUARD_JOINT_PROFILE']=str(OUT/'profile.json')
 before=state()
 run('prepare-gg-ag-'+name,[FG,'prepare',REPO,OUT]);run('prepare-sg-'+name,[FG,'sg-prepare',REPO,OUT]);run('prepare-tg-'+name,[TG,'prepare',REPO,OUT])
 base=json.loads((OUT/'binding.json').read_text()); cg=OUT/'codeguard';cg.mkdir(exist_ok=True)
 source=sha((REPO/'app.py').read_bytes()); binding=dict(base,sourceSnapshotDigest='sha256:'+source)
 argv=['lint','python',str(REPO),'--format=json','--file','app.py','--ruff-tool',str(RUFF)]
 producer=FIXED[str(NATIVE)]
 inv={'version':'codeguard.ruff-invocation/v1alpha1','packageVersion':'0.1.4','nativeSchema':'0.13.0','executable':str(NATIVE),'argv':argv,'runId':'controller-prepare-only','processExit':3,'producerSha256':producer}
 ctx={'version':'codeguard.ruff-context/v1alpha1','runId':inv['runId'],'binding':binding,'target':'app.py','producerSha256':producer,'startedAt':'2026-10-09T00:00:00Z','finishedAt':'2026-10-09T00:00:01Z'}
 contract={'apiVersion':'guard.partme.ai/v1alpha1','kind':'GuardContract','metadata':{'id':'ruff-f401','revision':'1'},'spec':{'rules':[{'id':'f401','enforcement':'enforce','assertion':{'type':'forbid_relation','subject':'python','predicate':'has','object':'unused-import'}}]}}
 mapping={'version':'codeguard.mapping/v1alpha1','entries':[{'source':{'kind':'finding','tool_id':'ruff','native_rule_id':'F401'},'rule_id':'f401'},{'source':{'kind':'gap','detail':'ruff_f401_scope_incomplete'},'rule_id':'f401'}]}
 def write(name,data):(cg/name).write_text(json.dumps(data,separators=(',',':')))
 for name,data in [('prepare-invocation.json',inv),('prepare-context.json',ctx),('contract-input.json',contract),('mapping.json',mapping)]:write(name,data)
 def sidecar(invfile,ctxfile,nativefile):return [CG,'guard-project-ruff','--invocation',cg/invfile,'--native-report',cg/nativefile,'--context',cg/ctxfile,'--mapping',cg/'mapping.json','--contract',cg/'contract-input.json']
 assert not (cg/'not-yet-executed.json').exists()
 p=run('cg-prepare-before-native',sidecar('prepare-invocation.json','prepare-context.json','not-yet-executed.json'),4)
 (cg/'prepare.stdout.json').write_bytes(p.stdout);(cg/'prepare.stderr.json').write_bytes(p.stderr)
 assert json.loads(p.stdout)['envelope']['decision'] is None
 run('freeze-cg-expectation',[FG,'cg-prepare',REPO,OUT]);run('freeze-fg-before-results',[FG,'freeze',REPO,OUT])
 # Save controller expectations before any native execution or completed specialist result.
 protected={str(p.relative_to(OUT)):sha(p.read_bytes()) for p in OUT.rglob('*.json')}
 p=run('actual-native-codeguard',[NATIVE,*argv],3);(cg/'native.json').write_bytes(p.stdout);(cg/'native.stderr').write_bytes(p.stderr)
 native=json.loads(p.stdout);assert native['schema_version']=='0.13.0' and native['exit_code']==3;inv=dict(inv,runId=native['run_id']);ctx=dict(ctx,runId=native['run_id'])
 write('invocation.json',inv);write('context.json',ctx)
 p=run('actual-cg-projection',sidecar('invocation.json','context.json','native.json'));(cg/'bundle.json').write_bytes(p.stdout);(cg/'projection.stderr').write_bytes(p.stderr)
 bundle=json.loads(p.stdout);assert bundle['nativeExit']==3;assert bundle['artifacts']['domain'].encode()==(cg/'native.json').read_bytes()
 write('envelope.json',bundle['envelope'])
 for name in ['contract','facts','report','domain']:(cg/(name+'.json')).write_bytes(bundle['artifacts'][name].encode())
 assert (cg/'contract.json').read_bytes()==(cg/'prepared-contract.json').read_bytes()
 assert bundle['envelope']['coverage']['requiredScopes']==json.loads((cg/'expected.json').read_text())['required_scopes']
 tg=OUT/'testguard'; command=['cargo','test','--locked','--offline','--manifest-path',str(REPO/'Cargo.toml'),'-p','cross-guard-app','--lib','tests::required_answer','--']
 p=run('actual-cargo-discovery',command+['--list','--format','terse']);(tg/'inventory.raw').write_bytes(p.stdout);(tg/'inventory.stderr').write_bytes(p.stderr)
 p=run('actual-cargo-run',command+['--format','pretty','--test-threads=1']);(tg/'stdout.raw').write_bytes(p.stdout);(tg/'stderr.raw').write_bytes(p.stderr)
 (tg/'capture.json').write_text(json.dumps({'exit_code':p.returncode,'interrupted':False,'profile':'trusted fixed fixture only','command':command+['--format','pretty','--test-threads=1']},indent=2)+'\n')
 run('actual-tg-producer',[TG,'finish',REPO,OUT]);run('actual-sg-producer',[FG,'sg-finish',REPO,OUT]);run('actual-ag-gg-producers',[FG,'finish',REPO,OUT])
 assert not run('git-status-after',['/usr/bin/git','status','--porcelain']).stdout
 for path,digest in protected.items():assert sha((OUT/path).read_bytes())==digest,('controller input mutated',path)
 for name in ['archguard','codeguard','testguard','gitguard']:
  e=json.loads((OUT/name/'envelope.json').read_text());assert e['binding']['candidateOid']==OID;assert e['binding']['baseOid']==profile['base'];assert e['decision']=='ALLOW';assert e['coverage']['status']=='complete'

 assert before==state(),('producer source write',name)
 assert sha((REPO/'baseline.json').read_bytes())==project_digest
 provider_hashes[case_name]={str(p.relative_to(OUT)):sha(p.read_bytes()) for p in OUT.rglob('*') if p.is_file()}

# Fixed records are captured before consumer execution; originals are not edited.
write(DEST/'producer-manifest.json',{'authority_profile':'explicit-local-fixture','production_authenticated':False,'flowguard_generator_runtime_observation':source_observation(ROOT),'profiles':profiles,'project_digest':project_digest,'artifacts':provider_hashes,'commands':records,'tools':{str(p):sha(Path(p).read_bytes()) for p in [FG,TG,CONSUMER,NATIVE,CG,RUFF]},'sources':{str(p.relative_to(ROOT)):sha(p.read_bytes()) for p in [HERE/'capture.py',HERE/'consume.py',ROOT/'examples/joint_invalidation.rs',ROOT/'examples/generate_joint.rs',ROOT/'examples/joint_support/mod.rs',HERE/'helper/src/main.rs',HERE/'helper/Cargo.toml',HERE/'helper/Cargo.lock']},'dependencies_runtime_observation':{name:source_observation(ROOT.parent/name) for name in ['guardengine','specguard','archguard','gitguard','testguard']},'testguard_helper_source':'2d2189f9b083ff719667613a7b0f44c3b78c0088','helper_guardengine_source':'b0e4b445e1a20a9354d55ebb41bbedaf16ec8a74','native_codeguard_source':'f5661d51dd1b2629f3b87b726fb09a9060ce7958','codeguard_projection_source':'72d80ae453c0380ad7061c10847595c5a4ac1736','flowguard_consumer_source':'56a2c9576caf382a6bb5f716c707c77c94606599'})
p=subprocess.run([sys.executable,str(HERE/'consume.py'),str(DEST)],capture_output=True,env=ENV,timeout=60)
assert p.returncode==0,p.stderr.decode(errors='replace')
sys.stdout.buffer.write(p.stdout)

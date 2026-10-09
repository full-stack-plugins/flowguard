#!/usr/bin/env python3
"""Consume freshly generated artifacts; never rewrite a provider envelope."""
import copy,hashlib,json,os,shutil,subprocess,sys
from pathlib import Path
DEST=Path(sys.argv[1]).resolve();repo=DEST/'repository'
binary=Path('/workspace/guard-implementation-ledger/fixed-tools/flowguard-independent-56a2c95/flowguard')
NOW=2_000_000_000
NAMES=['archguard','codeguard','testguard','gitguard','specguard']
raw_digest=lambda b:'sha256:'+hashlib.sha256(b).hexdigest()
def read(path):return json.loads(path.read_bytes())
def write(path,value):path.write_text(json.dumps(value,separators=(',',':')))
def state(repo):return {str(p.relative_to(repo)):raw_digest(p.read_bytes()) for p in repo.rglob('*') if p.is_file()}
def prepare(source):
    b = read(source / 'binding.json')
    scopes = read(source / 'scopes.json')
    policies = read(source / 'policies.json')
    frozen = read(source / 'frozen.json')
    baseline = read(source / 'specguard/baseline.json')
    # This is the explicitly declared protected LOCAL fixture issuer descriptor.
    # Validity is intersected exactly as in repaired bridge f769cca, not inferred
    # from a successful report. It is not an online production authority.
    baseline_auth = {'issued_at': NOW - 10, 'expires_at': NOW + 100, 'revoked': False}
    def authority(auth):
        producers = []
        for name in NAMES:
            expected = read(source / name / 'expected.json')
            envelope = read(source / name / 'envelope.json')
            # Rust envelope canonical bytes use the original struct field ordering;
            # retained compact producer bytes are already emitted in that ordering.
            receipt = {'principal': 'fixture-ci', 'producer': expected['producer'], 'envelope_digest': raw_digest((source / name / 'envelope.json').read_bytes()), 'issued_at': 0, 'expires_at': 9223372036854775807, 'revoked': False}
            if name == 'specguard':
                receipt['issued_at'] = max(receipt['issued_at'], baseline['effectiveFrom'], auth['issued_at'])
                receipt['expires_at'] = min(receipt['expires_at'], baseline['expiresAt'], auth['expires_at'])
                receipt['revoked'] = auth['revoked']
            assert receipt['issued_at'] < receipt['expires_at']
            assert envelope['producer'] == expected['producer']
            producers.append(receipt)
        sg = policies[scopes['specguard']]
        approval = {'reference': 'fixture:actual-spec-review', 'principal': 'fixture-reviewer', 'purpose': 'review', 'action': 'commit', 'binding': sg['binding'], 'contract_digest': sg['contract_digest'], 'issued_at': NOW - 10, 'expires_at': 9223372036854775807, 'revoked': False}
        return {'version': 'flowguard.authority-fixture/v1alpha1', 'producers': producers, 'approvals': [approval]}
    request = {'version': 'flowguard.cli-request/v1alpha1', 'invocation': {'repo_candidates': [b['repoId']], 'task_candidates': [b['taskId']], 'worktree_id': b['worktreeId'], 'requirement_ids': b['requirementIds'], 'candidate_oid': b['candidateOid'], 'base_oid': b['baseOid']}, 'candidate': 'gitguard/candidate.json', 'frozen': 'frozen.json', 'frozen_digest': frozen['digest'], 'sources': 'sources.json', 'sources_digest': raw_digest((source / 'sources.json').read_bytes()), 'action': 'commit', 'started_at': '2033-05-18T03:33:20Z', 'finished_at': '2033-05-18T03:33:20Z', 'now': NOW, 'specialists': []}
    for name in NAMES:
        policy = policies[scopes[name]]
        request['specialists'].append({'scope': scopes[name], 'producer': policy['producer'], 'producer_principals': policy['producer_principals'], 'approval_principals': policy['approval_principals'], 'evidence': {field: name + '/' + filename for field, filename in [('envelope', 'envelope.json'), ('contract', 'contract.json'), ('facts', 'facts.json'), ('report', 'report.json')]}})
    return request,authority(baseline_auth)
requests={name:prepare(DEST/name) for name in ['A','B','M1','M2']}
initial={str(p.relative_to(DEST)):raw_digest(p.read_bytes()) for name in requests for p in (DEST/name).rglob('*') if p.is_file()}
worktrees={}
for name,(request,_) in requests.items():
 path=DEST/('worktree-'+name)
 if not path.exists():
  subprocess.run(['/usr/bin/git','clone','--quiet','--no-hardlinks',str(repo),str(path)],check=True,capture_output=True)
  subprocess.run(['/usr/bin/git','-C',str(path),'checkout','--detach',request['invocation']['candidate_oid']],check=True,capture_output=True)
 worktrees[name]=path
rows={}
def run(case,target,foreign=None,variant=None):
 repo=worktrees[target]
 source=DEST/target;req,auth=copy.deepcopy(requests[target])
 if foreign:
  old=DEST/foreign
  for name in NAMES:
   dst=source/('foreign-'+foreign)/name
   if not dst.exists():shutil.copytree(old/name,dst)
  for item in req['specialists']:
   item['evidence']={k:'foreign-'+foreign+'/'+v for k,v in item['evidence'].items()}
  auth['producers']+=copy.deepcopy(requests[foreign][1]['producers'])
 if variant=='expired':req['now']=NOW+200
 if variant=='revoked':auth['approvals'][0]['revoked']=True
 request_name='request-'+case+'.json';authority_name='authority-'+case+'.json'
 write(source/request_name,req);write(DEST/authority_name,auth)
 cmd=[str(binary),'gate','check','--repo',str(repo),'--input-root',str(source),'--request',request_name,'--request-digest',raw_digest((source/request_name).read_bytes()),'--run-id','joint-'+case,'--local-fixture-authority','--fixture-authority-root',str(DEST),'--fixture-authority',authority_name,'--fixture-authority-digest',raw_digest((DEST/authority_name).read_bytes())]
 before=state(repo);p=subprocess.run(cmd,capture_output=True,timeout=30);assert before==state(repo),case
 (DEST/(case+'.stdout.json')).write_bytes(p.stdout);(DEST/(case+'.stderr.json')).write_bytes(p.stderr)
 expected=4 if foreign or variant else 0
 assert p.returncode==expected,(case,p.returncode,p.stderr.decode(),p.stdout.decode())
 payload=json.loads(p.stdout);assert payload['execution_authorized'] is False
 assert payload['envelope']['decision']==(None if foreign or variant else 'ALLOW')
 rows[case]={'exit':p.returncode,'command':cmd,'stdout':raw_digest(p.stdout),'stderr':raw_digest(p.stderr)}
for name in requests:run(name,name)
for case,target,foreign in [('A_to_B','B','A'),('A_to_M1','M1','A'),('M1_to_M2','M2','M1')]:run(case,target,foreign)
run('A_expired','A',variant='expired')
run('B_revoked','B',variant='revoked')
assert all(raw_digest((DEST/name).read_bytes())==digest for name,digest in initial.items())
producer_manifest=read(DEST/'producer-manifest.json')
for case,artifacts in producer_manifest['artifacts'].items():
 for name,digest in artifacts.items():assert hashlib.sha256((DEST/case/name).read_bytes()).hexdigest()==digest,(case,name)
policy=read(DEST/'A/specguard/contract.json');policy['metadata']['revision']='2';write(DEST/'rules-revision2.json',policy)
write(DEST/'analyzer-revision2.json',{'guard':'SpecGuard','analyzerId':'specguard.structural','analyzerVersion':'2'})
# Joint publication invalidation is verified by the same native library helper.
write(DEST/'project-baseline-revision2.json',{'version':'joint.project/v1','project':'shared-02','revision':2})
helper=Path(os.environ.get('FLOWGUARD_JOINT_INVALIDATION',Path(__file__).resolve().parents[3]/'target/debug/examples/joint_invalidation'))
p=subprocess.run([str(helper),str(DEST)],capture_output=True,timeout=30)
assert p.returncode==0,p.stderr.decode()
(DEST/'invalidation.stdout.json').write_bytes(p.stdout);(DEST/'invalidation.stderr').write_bytes(p.stderr)
check=json.loads(p.stdout);assert check['shared_project_invalidation']
report={'profile':'flowguard.joint-local/v1','cases':rows,'shared_project_invalidation':True,'source_unchanged':True,'production_authenticated':False,'execution_authorized':False,'invalidation':check,'consumer_sha256':raw_digest(binary.read_bytes()),'invalidation_helper_sha256':raw_digest(helper.read_bytes()),'invalidation_command':[str(helper),str(DEST)]}
write(DEST/'result.json',report);print(json.dumps(report,separators=(',',':')))

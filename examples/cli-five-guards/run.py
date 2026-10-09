#!/usr/bin/env python3
"""Fixed local fixture CLI exercise. No install, host credentials or publishing."""
import argparse, copy, hashlib, json, os, shutil, subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent
CORPUS = HERE.parents[1] / 'fixtures/providers/approval-bridge/actual'
NOW = 2_000_000_000
NAMES = ['archguard', 'codeguard', 'testguard', 'gitguard', 'specguard']

def raw_digest(data):
    return 'sha256:' + hashlib.sha256(data).hexdigest()

def write(path, value):
    path.write_text(json.dumps(value, separators=(',', ':')))

def read(path):
    return json.loads(path.read_bytes())

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--flowguard', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    binary = args.flowguard.resolve()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    source = out / 'input'
    shutil.copytree(CORPUS, source)
    provenance = read(source / 'PROVENANCE.json')
    for name, digest in provenance['artifacts'].items():
        assert hashlib.sha256((source / name).read_bytes()).hexdigest() == digest, name
    repo = out / 'repository'
    env = dict(os.environ, GIT_CONFIG_NOSYSTEM='1', GIT_CONFIG_GLOBAL='/dev/null', GIT_OPTIONAL_LOCKS='0')
    def git(*argv):
        result = subprocess.run(['/usr/bin/git', '-C', str(repo), *argv], env=env, capture_output=True, timeout=30)
        assert result.returncode == 0, result.stderr.decode(errors='replace')
        return result.stdout
    result = subprocess.run(['/usr/bin/git', 'clone', '--quiet', str(source / 'source.bundle'), str(repo)], env=env, capture_output=True, timeout=30)
    assert result.returncode == 0, result.stderr.decode(errors='replace')
    original_source_hashes = {str(p.relative_to(source)): raw_digest(p.read_bytes()) for p in source.rglob('*') if p.is_file()}
    def repo_state():
        files = [(name.decode(), raw_digest((repo / name.decode()).read_bytes())) for name in git('ls-files', '-z').split(b'\0') if name]
        return raw_digest(json.dumps([git('for-each-ref', '--format=%(refname) %(objectname)').hex(), git('rev-parse', 'HEAD').hex(), (repo / '.git/index').read_bytes().hex(), files, git('status', '--porcelain=v1').hex()], separators=(',', ':')).encode())
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
    rows = []
    required = None
    cases = ['normal', 'missing', 'revoked', 'expired', 'cancel', 'bad-evidence', 'orphan-source', 'orphan-digest', 'null-source', 'wrong-digest', 'unknown-profile', 'unknown-profile-field', 'candidate-drift']
    for case in cases:
        req = copy.deepcopy(request)
        auth = authority(baseline_auth)
        extra = []
        prebinding = case in ['orphan-source', 'orphan-digest', 'null-source', 'wrong-digest', 'unknown-profile', 'unknown-profile-field', 'candidate-drift']
        expected_code = 0 if case == 'normal' else (2 if case == 'missing' else 4)
        if case == 'missing':
            req['specialists'][-1]['evidence'] = None
        elif case == 'revoked':
            auth['approvals'][0]['revoked'] = True
        elif case == 'expired':
            auth = authority(dict(baseline_auth, expires_at=NOW + 1))
            req['now'] = NOW + 2
        elif case == 'cancel':
            extra = ['--cancel']
        elif case == 'bad-evidence':
            req['specialists'][0]['evidence']['report'] = 'absent-report.json'
        elif case == 'orphan-source':
            del req['sources_digest']
        elif case == 'orphan-digest':
            del req['sources']
        elif case == 'null-source':
            req['sources'] = None
        elif case == 'wrong-digest':
            req['sources_digest'] = 'sha256:' + 'f' * 64
        elif case in ['unknown-profile', 'unknown-profile-field']:
            profile = read(source / 'sources.json')
            if case == 'unknown-profile': profile['version'] = 'flowguard.specialist-sources/v99'
            else: profile['unexpected'] = True
            write(source / 'bad-sources.json', profile)
            req['sources'] = 'bad-sources.json'; req['sources_digest'] = raw_digest((source / 'bad-sources.json').read_bytes())
        elif case == 'candidate-drift':
            # External protected controller selects a real new commit. No assumption
            # requires an arbitrary requested/synthetic candidate to equal HEAD.
            git('-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid', 'commit', '--allow-empty', '-qm', 'controller selects later candidate')
            req['invocation']['candidate_oid'] = git('rev-parse', 'HEAD').decode().strip()
            assert req['invocation']['candidate_oid'] != b['candidateOid']
        request_name = 'request-' + case + '.json'
        authority_name = 'authority-' + case + '.json'
        write(source / request_name, req)
        write(out / authority_name, auth)
        cmd = [str(binary), 'gate', 'check', '--repo', str(repo), '--input-root', str(source), '--request', request_name, '--request-digest', raw_digest((source / request_name).read_bytes()), '--run-id', 'cli-five-' + case]
        cmd += ['--local-fixture-authority', '--fixture-authority-root', str(out), '--fixture-authority', authority_name, '--fixture-authority-digest', raw_digest((out / authority_name).read_bytes())]
        cmd += extra
        before = repo_state()
        result = subprocess.run(cmd, capture_output=True, timeout=30, env=env)
        assert before == repo_state(), ('repository mutated', case)
        (out / (case + '.stdout.json')).write_bytes(result.stdout)
        (out / (case + '.stderr.json')).write_bytes(result.stderr)
        assert result.returncode == expected_code, (case, result.returncode, result.stderr.decode(errors='replace'))
        if prebinding:
            assert not result.stdout, case
            diagnostic = json.loads(result.stderr)
            assert diagnostic['code'] == 'flowguard.input_unresolved'
            decision = None
        else:
            payload = json.loads(result.stdout)
            assert payload['execution_authorized'] is False
            assert payload['authority_profile'] == 'local_fixture'
            envelope = payload['envelope']; decision = envelope['decision']
            if case == 'normal':
                assert decision == 'ALLOW' and envelope['coverage']['status'] == 'complete'
                required = envelope['coverage']['requiredScopes']
                assert 'flowguard.sources:' + read(source / 'sources.json')['digest'] in required
            elif case == 'missing':
                assert decision == 'BLOCK' and envelope['coverage']['status'] == 'partial'
            else:
                assert decision is None
                assert envelope['runStatus'] == ('cancelled' if case == 'cancel' else 'error')
            assert envelope['coverage']['requiredScopes'] == required, case
        rows.append({'case': case, 'exit': result.returncode, 'decision': decision, 'prebinding': prebinding, 'command': cmd})
    for name, digest in original_source_hashes.items():
        assert raw_digest((source / name).read_bytes()) == digest, ('original artifact changed', name)
    assert read(source / 'specguard/original.json')['envelope']['decision'] == 'REQUIRE_APPROVAL'
    assert read(source / 'specguard/envelope.json')['decision'] == 'REQUIRE_APPROVAL'
    assert (source / 'codeguard/domain.json').read_bytes() == (source / 'codeguard/native.json').read_bytes()
    native = [c for c in provenance['commands'] if c['label'] == 'actual-native-codeguard']
    assert native[0]['exit'] == 3
    write(out / 'summary.json', rows)
    print(json.dumps([{k: v for k, v in row.items() if k != 'command'} for row in rows], separators=(',', ':')))

if __name__ == '__main__':
    main()

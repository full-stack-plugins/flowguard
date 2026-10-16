"""只读原生阶段适配：缺失→修复→复验，不授予审批。"""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

BIN = Path(os.environ.get('FLOWGUARD_BIN', 'target/debug/flowguard' + ('.exe' if os.name == 'nt' else ''))).resolve()
PLUGIN = Path(__file__).resolve().parents[2] / 'flowguard-plugin'
sys.path.insert(0, str(PLUGIN / 'scripts'))
from flowguard_lib import stage_docs, registry


class StageCheck(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.previous = os.environ.get('FLOWGUARD_STATE_HOME')
        os.environ['FLOWGUARD_STATE_HOME'] = str(self.root / 'host-state')
        self.addCleanup(self.restore)

    def restore(self):
        if self.previous is None:
            os.environ.pop('FLOWGUARD_STATE_HOME', None)
        else:
            os.environ['FLOWGUARD_STATE_HOME'] = self.previous

    def request(self, **kwargs):
        return dict(project=str(self.root), flowguard_root=str(PLUGIN), python=sys.executable, task_id='demo', **kwargs)

    def cli(self, request):
        mode = request.pop('mode', 'check')
        args = [str(BIN), 'stage', mode]
        for key, value in request.items():
            args.extend(['--' + key.replace('_', '-'), value])
        result = subprocess.run(args, capture_output=True, encoding='utf-8', timeout=45)
        self.assertIn(result.returncode, (0, 2, 3), result.stderr)
        return json.loads(result.stdout)

    def mcp(self, request):
        messages = [dict(jsonrpc='2.0', id=1, method='initialize', params=dict(protocolVersion='2024-11-05', capabilities={}, clientInfo=dict(name='tests', version='1'))),
                    dict(jsonrpc='2.0', method='notifications/initialized'),
                    dict(jsonrpc='2.0', id=2, method='tools/call', params=dict(name='flowguard_stage_check', arguments=request))]
        result = subprocess.run([str(BIN), 'serve', '--mcp'], input=''.join(json.dumps(m) + '\n' for m in messages), capture_output=True, encoding='utf-8', timeout=45)
        reply = [json.loads(row) for row in result.stdout.splitlines() if json.loads(row).get('id') == 2][0]
        return json.loads(reply['result']['content'][0]['text'])

    def snapshot(self):
        return {str(p.relative_to(self.root)): hashlib.sha256(p.read_bytes()).hexdigest() for p in self.root.rglob('*') if p.is_file()}

    def same(self, request):
        before = self.snapshot()
        result = self.cli(dict(request))
        self.assertEqual(result, self.mcp(request))
        self.assertEqual(before, self.snapshot())
        self.assertEqual(result['approval'], 'required')
        self.assertNotEqual(result['decision'], 'allow')
        return result

    def test_missing_fix_and_ready_still_requires_approval(self):
        request = self.request(action='code_write')
        self.assertEqual(self.same(request)['decision'], 'unverified')
        stage_docs.ensure(self.root, dict(task_id='demo', spec_system='none'))
        self.assertEqual(self.same(request)['decision'], 'deny')
        for stage in list(registry.ARTIFACTS)[:7]:
            stage_docs.advance(self.root, 'demo', stage, 'skipped', approval_ref='fixture:untrusted-local-text', reason='isolated test fixture')
        result = self.same(request)
        self.assertEqual(result['decision'], 'requires_approval')
        self.assertEqual(result['missing'], [])
        for action in ('git_commit', 'release'):
            self.assertEqual(self.same(self.request(action=action))['decision'], 'deny')
        path = stage_docs.path_for(self.root, 'demo', '01-requirements')
        path.write_text(path.read_text(encoding='utf-8') + '\nchanged\n', encoding='utf-8')
        self.assertEqual(self.same(request)['decision'], 'deny')
        self.assertFalse(any(p.name == '.flowguard' for p in self.root.rglob('*')))

    def test_malformed_status_repair_and_no_actor(self):
        stage_docs.ensure(self.root, dict(task_id='demo'))
        path = stage_docs.path_for(self.root, 'demo', '01-requirements')
        good = path.read_text(encoding='utf-8')
        path.write_text('invalid', encoding='utf-8')
        self.assertEqual(self.same(self.request(mode='status'))['decision'], 'unverified')
        path.write_text(good, encoding='utf-8')
        self.assertEqual(self.same(self.request(mode='status'))['decision'], 'observed')
        result = subprocess.run([str(BIN), 'stage', 'check', '--actor', 'user'], capture_output=True)
        self.assertEqual(result.returncode, 4)

    def test_oversized_stage_is_unverified_then_repair(self):
        stage_docs.ensure(self.root, dict(task_id='demo'))
        path = stage_docs.path_for(self.root, 'demo', '01-requirements')
        original = path.read_bytes()
        path.write_bytes(original + b'\n' + b'x' * (1024 * 1024))
        self.assertEqual(self.same(self.request(mode='status'))['decision'], 'unverified')
        path.write_bytes(original)
        self.assertEqual(self.same(self.request(mode='status'))['decision'], 'observed')

    def test_external_docs_link_is_unverified_then_repair(self):
        stage_docs.ensure(self.root, dict(task_id='demo'))
        with tempfile.TemporaryDirectory() as outside:
            link = self.root / 'docs'
            source = Path(outside) / 'docs'
            link.rename(source)
            if os.name == 'nt':
                subprocess.run(['cmd', '/c', 'mklink', '/J', str(link), str(source)], check=True, capture_output=True)
            else:
                link.symlink_to(source, target_is_directory=True)
            try:
                self.assertEqual(self.same(self.request(mode='status'))['decision'], 'unverified')
            finally:
                if os.name == 'nt':
                    os.rmdir(link)
                else:
                    link.unlink()
                source.rename(link)
        self.assertEqual(self.same(self.request(mode='status'))['decision'], 'observed')


if __name__ == '__main__':
    unittest.main()

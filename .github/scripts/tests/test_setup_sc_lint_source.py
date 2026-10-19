"""Immutable checkout, provenance-mode and platform contracts for source setup."""
import importlib.util
import contextlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock
import yaml
import shutil

SCRIPT = Path(__file__).resolve().parents[1] / 'setup_sc_lint_source.py'
spec = importlib.util.spec_from_file_location('setup_source', SCRIPT)
setup = importlib.util.module_from_spec(spec)
spec.loader.exec_module(setup)


class SourceSetupTests(unittest.TestCase):
    def test_refs_are_full_immutable_sha_only(self):
        for ref in ('main', 'v0.6.0', 'HEAD', 'abc123', 'a'*39, 'z'*40, 'a'*40+'\n', '--help', None):
            with self.subTest(ref=ref), self.assertRaises(ValueError):
                setup.revision(ref)
        self.assertEqual(setup.revision('a'*40), 'a'*40)

    def test_execution_smoke_retains_reported_findings(self):
        result = {'ok': True, 'data': {'status': 'fail', 'diagnostics': ['visible finding']}}
        with contextlib.redirect_stdout(io.StringIO()) as output:
            setup.report_lint_smoke(result)
        self.assertIn('visible finding', output.getvalue())
        self.assertIn('reported lint status=fail', output.getvalue())
        for value in ({'ok': False}, {'ok': True, 'error': {'code': 'CLI.CONFIG_ERROR'}}):
            with contextlib.redirect_stdout(io.StringIO()), self.assertRaisesRegex(ValueError, 'root discovery failed'):
                setup.report_lint_smoke(value)

    def test_paths_match_actual_unix_and_windows_helper_lookup(self):
        root = Path('consumer/.sc-lint/venv')
        self.assertEqual(setup.python_path(root, False), root / 'bin/python3')
        self.assertEqual(setup.python_path(root, True), root / 'Scripts/python.exe')

    def test_manifest_selection_is_opt_in_and_conflicts_fail(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'manifest.toml'
            self.assertEqual(setup.selected_revision('', path), '')
            path.write_text('[project]\nsc_lint_source_revision="' + 'a'*40 + '"\n')
            self.assertEqual(setup.selected_revision('', path), 'a'*40)
            self.assertEqual(setup.selected_revision('a'*40, path), 'a'*40)
            with self.assertRaisesRegex(ValueError, 'differs'):
                setup.selected_revision('b'*40, path)
            for value in ('"main"', '""', 'false', '12'):
                path.write_text('[project]\nsc_lint_source_revision=' + value)
                with self.assertRaises(ValueError):
                    setup.selected_revision('', path)

    def test_real_git_checkout_must_match_exact_requested_commit(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            origin = root / 'origin'
            origin.mkdir()
            setup.run(['git', 'init', '-q'], origin)
            (origin/'Cargo.lock').write_text('reviewed source')
            setup.run(['git', 'add', '.'], origin)
            setup.run(['git', '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.test', 'commit', '-qm', 'source'], origin)
            sha = setup.run(['git', 'rev-parse', 'HEAD'], origin)
            target = root / 'checkout'
            setup.checkout(str(origin), sha, target)
            setup.verify_checkout(target, sha)
            with self.assertRaisesRegex(ValueError, 'does not match'):
                setup.verify_checkout(target, '0'*40)
            (target/'Cargo.lock').write_text('changed')
            with self.assertRaises(subprocess.CalledProcessError):
                setup.verify_checkout(target, sha)
            with self.assertRaises(subprocess.CalledProcessError):
                setup.checkout(str(origin), '0'*40, root/'missing')

    def test_release_cannot_silently_reuse_source_install(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root/'.sc-lint').mkdir()
            (root/'.sc-lint/source-install.json').write_text('{}')
            env = dict(os.environ, GITHUB_WORKSPACE=str(root), GITHUB_ENV=str(root/'env'), SC_LINT_SOURCE_REVISION_INPUT='')
            result = subprocess.run(
                [sys.executable, SCRIPT, 'resolve'],
                env=env,
                stdin=subprocess.DEVNULL,
                capture_output=True,
                text=True,
                timeout=30,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('cannot reuse', result.stderr)
            self.assertFalse((root/'env').exists())

    def test_default_action_resolution_does_not_invoke_source_helper(self):
        action = SCRIPT.parents[1] / 'actions/setup-sc-lint/action.yml'
        steps = yaml.safe_load(action.read_text())['runs']['steps']
        for label, shell in (('Unix', 'bash'), ('Windows', sys.executable)):
            if not shutil.which(shell):
                continue
            with self.subTest(platform=label), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root/'release').mkdir()
                (root/'release/publish-artifacts.toml').write_text('[project]\nname="old-consumer"\n')
                # Any source helper invocation fails in default release mode.
                helper = root/'.github/scripts/setup_sc_lint_source.py'
                helper.parent.mkdir(parents=True)
                helper.write_text('raise RuntimeError("default release must not invoke source helper")')
                step = next(x for x in steps if x['name'] == f'Resolve immutable sc-lint source ({label})')
                script = root / ('resolve.py' if label == 'Windows' else 'resolve.sh')
                script.write_text(step['run'])
                env = dict(os.environ, GITHUB_ENV=str(root/'env'), SC_LINT_SOURCE_REVISION_INPUT='')
                cmd = [shell, str(script)]
                result = subprocess.run(
                    cmd,
                    cwd=root,
                    env=env,
                    stdin=subprocess.DEVNULL,
                    capture_output=True,
                    text=True,
                    timeout=30,
                )
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual((root/'env').read_text().strip(), 'SC_LINT_SOURCE_REVISION=')

    def test_real_subprocess_timeout_fails_with_command_context(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(RuntimeError, 'timed out.*sleep') as caught:
                setup.run([sys.executable, '-c', 'import time; time.sleep(5)'], Path(directory), timeout=0.05)
            self.assertIsInstance(caught.exception.__cause__, subprocess.TimeoutExpired)
        self.assertEqual(setup.BUILD_TIMEOUT_SECONDS, 1800)
        self.assertLess(setup.PROBE_TIMEOUT_SECONDS, setup.SETUP_TIMEOUT_SECONDS)

    def test_existing_python_environment_rejected_before_fetch(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root/'.sc-lint/venv').mkdir(parents=True)
            with self.assertRaisesRegex(ValueError, 'fresh consumer'):
                setup.install('example/unavailable', 'a'*40, root, root)
            self.assertEqual(list(root.glob('sc-lint-source-*')), [])

    def test_source_install_record_contains_provenance_and_binary_digests(self):
        revision = 'a' * 40
        version = '0.7.3'

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            workspace = root / 'consumer'
            workspace.mkdir()
            (workspace / '.sc-lint').mkdir()

            def fake_checkout(url, sha, destination):
                self.assertEqual(url, 'https://github.com/example/sc-lint.git')
                self.assertEqual(sha, revision)
                destination.mkdir()

            def fake_run(command, cwd, env=None, timeout=setup.SETUP_TIMEOUT_SECONDS):
                command = [str(value) for value in command]
                if command[:2] == ['cargo', 'metadata']:
                    packages = [
                        {'name': name, 'version': version}
                        for name in (*setup.BINARIES, 'sc-lint-py')
                    ]
                    return json.dumps({'packages': packages})
                if command[:2] == ['cargo', 'build']:
                    target = Path(env['CARGO_TARGET_DIR']) / 'release'
                    target.mkdir(parents=True)
                    suffix = '.exe' if setup.os.name == 'nt' else ''
                    for name in setup.BINARIES:
                        (target / (name + suffix)).write_bytes(name.encode())
                    return ''
                if command[:3] == [sys.executable, '-m', 'venv']:
                    python = setup.python_path(Path(command[3]))
                    python.parent.mkdir(parents=True)
                    python.write_text('', encoding='utf-8')
                    return ''
                if len(command) >= 3 and command[1:3] == ['-m', 'maturin']:
                    output = Path(command[command.index('--out') + 1])
                    output.mkdir()
                    (output / 'sc_lint-0.7.3-py3-none-any.whl').write_bytes(b'wheel')
                    return ''
                if command[1:] == ['version', '--json']:
                    return json.dumps({'ok': True, 'data': {'version': version, 'status': 'pass'}})
                if command[1:3] == ['-I', '-c']:
                    venv = workspace / '.sc-lint/venv'
                    return json.dumps({'version': version, 'path': str(venv / 'lib/sc_lint/__init__.py')})
                return ''

            with mock.patch.object(setup, 'checkout', fake_checkout), \
                 mock.patch.object(setup, 'verify_checkout'), \
                 mock.patch.object(setup, 'run', fake_run):
                record = setup.install('example/sc-lint', revision, workspace, root)

            persisted = json.loads((workspace / '.sc-lint/source-install.json').read_text())
            self.assertEqual(persisted, record)
            self.assertEqual(record['source_revision'], revision)
            self.assertEqual(record['version'], version)
            self.assertEqual(record['wheel_sha256'], setup.digest(Path(record['wheel'])))
            self.assertEqual(set(record['binaries']), set(setup.BINARIES))
            for name, checksum in record['binaries'].items():
                self.assertEqual(checksum, setup.digest(Path(record['binary_directory']) / name))


if __name__ == '__main__':
    unittest.main()

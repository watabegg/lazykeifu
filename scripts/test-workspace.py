#!/usr/bin/env python3
"""Exercise the real TUI against throwaway repositories (Linux, requires script)."""
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import time

BINARY = Path(__file__).resolve().parents[1] / 'target/debug/lazykeifu'


def git(root, *args, check=True):
    return subprocess.run(['git', *args], cwd=root, check=check, capture_output=True, text=True)


def init(root):
    git(root, 'init', '-b', 'main')
    git(root, 'config', 'user.name', 'Workspace Test')
    git(root, 'config', 'user.email', 'workspace@example.invalid')
    git(root, 'config', 'commit.gpgsign', 'false')
    hooks = root / '.git' / 'test-hooks'
    hooks.mkdir()
    git(root, 'config', 'core.hooksPath', str(hooks))
    return hooks


class Tui:
    def __init__(self, root, config, slow_diff=False):
        with socket.socket() as sock:
            sock.bind(('127.0.0.1', 0))
            self.port = sock.getsockname()[1]
        env = dict(os.environ, XDG_CONFIG_HOME=str(config), GIT_CONFIG_NOSYSTEM='1', GIT_CONFIG_GLOBAL='/dev/null')
        # All interpolated values are generated paths / integers; quote the binary path.
        import shlex
        if slow_diff:
            import shutil
            wrapper = config.parent / 'slow-git'
            wrapper.mkdir(exist_ok=True)
            executable = shutil.which('git')
            (wrapper / 'git').write_text('#!/bin/sh\ncase "$*" in *"--no-pager diff"*) sleep 0.2;; esac\nexec ' + shlex.quote(executable) + ' "$@"\n')
            (wrapper / 'git').chmod(0o755)
            env['PATH'] = str(wrapper) + os.pathsep + env['PATH']
        command = f'{shlex.quote(str(BINARY))} --debug-listen 127.0.0.1:{self.port} --log-file {shlex.quote(str(config.parent / "tui-test.log"))}'
        self.proc = subprocess.Popen(['script', '-qec', command, '/dev/null'], cwd=root, env=env,
                                     stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        try:
            self.wait(lambda: self.state()['mode'] == 'normal')
        except BaseException:
            self.close()
            raise

    def request(self, **request):
        with socket.create_connection(('127.0.0.1', self.port), timeout=3) as sock:
            sock.sendall((json.dumps(request) + '\n').encode())
            result = json.loads(sock.makefile().readline())
        assert result['ok'], result
        return result

    def state(self):
        return self.request(cmd='state')

    def keys(self, keys):
        return self.request(cmd='keys', keys=keys)

    def screen(self, width=120, height=32):
        return self.request(cmd='dump', width=width, height=height)['screen']

    def wait(self, predicate):
        deadline = time.monotonic() + 8
        while time.monotonic() < deadline:
            try:
                if predicate():
                    return
            except (ConnectionRefusedError, ConnectionResetError):
                if self.proc.poll() is not None:
                    raise AssertionError(f'TUI exited: {self.proc.returncode}')
            time.sleep(.05)
        raise AssertionError('Timed out waiting for TUI state')

    def loaded(self):
        self.wait(lambda: not self.state()['workspace_preview_loading'])

    def close(self):
        if self.proc.poll() is None:
            try:
                self.keys('q')
                self.proc.wait(timeout=2)
            except (OSError, subprocess.TimeoutExpired, AssertionError, json.JSONDecodeError):
                self.proc.terminate()
                self.proc.wait(timeout=3)


def run():
    with tempfile.TemporaryDirectory(prefix='lazykeifu-e2e-') as temp:
        root = Path(temp) / 'repo'
        root.mkdir()
        hooks = init(root)
        config = Path(temp) / 'config'
        (config / 'lazykeifu').mkdir(parents=True)
        (config / 'lazykeifu/config.toml').write_text('[refresh]\nauto_fetch=false\nauto_refresh=false\n')
        (root / 'a.txt').write_text('base\n')
        git(root, 'add', 'a.txt')
        git(root, 'commit', '-m', 'initial')
        original = git(root, 'rev-parse', 'HEAD').stdout.strip()
        (root / 'a.txt').write_text('staged-only\n')
        git(root, 'add', 'a.txt')
        (root / 'a.txt').write_text('worktree-only\n')
        for index in range(60):
            (root / f'z{index:02}.txt').write_text(f'untracked {index}\n')
        app = Tui(root, config, slow_diff=True)
        try:
            app.loaded()
            assert git(root, 'show', ':a.txt').stdout == 'staged-only\n'
            assert app.state()['workspace_file_count'] == 61
            assert '+worktree-only' in app.screen()
            if os.environ.get('LAZYKEIFU_TEST_ARTIFACTS'):
                evidence = Path(os.environ['LAZYKEIFU_TEST_ARTIFACTS'])
                evidence.mkdir(parents=True, exist_ok=True)
                (evidence / 'workspace-wide.txt').write_text(app.screen())
            app.keys('v')
            app.wait(lambda: app.state()['workspace_preview_loading'])
            app.keys('v')  # return to the original side while the other diff is in flight
            app.loaded()
            assert '+worktree-only' in app.screen()
            app.keys('v')
            app.loaded()
            screen = app.screen()
            assert '+staged-only' in screen and '+worktree-only' not in screen, screen
            app.keys('<space>')
            app.loaded()
            assert git(root, 'diff', '--cached', '--quiet', check=False).returncode == 0
            assert (root / 'a.txt').read_text() == 'worktree-only\n'
            app.keys('<space>')
            app.loaded()
            assert 'worktree-only' in git(root, 'show', ':a.txt').stdout
            hook = hooks / 'pre-commit'
            hook.write_text('#!/bin/sh\necho hook-rejected >&2\nexit 1\n')
            hook.chmod(0o755)
            app.keys('c r e t a i n e d <enter>')
            app.wait(lambda: not app.state()['workspace_busy'])
            assert app.state()['mode'] == 'input'
            assert 'retained' in app.screen()
            assert 'hook-rejected' in app.screen()
            assert git(root, 'rev-parse', 'HEAD').stdout.strip() == original
            hook.write_text('#!/bin/sh\nexit 0\n')
            signer = hooks / 'test-signer'
            signer.write_text('#!/bin/sh\ntouch .git/signer-invoked\necho signer-rejected >&2\nexit 1\n')
            signer.chmod(0o755)
            git(root, 'config', 'gpg.program', str(signer))
            git(root, 'config', 'commit.gpgsign', 'true')
            app.keys('<enter>')
            app.wait(lambda: not app.state()['workspace_busy'])
            assert (root / '.git/signer-invoked').exists()
            assert app.state()['mode'] == 'input'
            assert 'retained' in app.screen()
            assert git(root, 'rev-parse', 'HEAD').stdout.strip() == original
            git(root, 'config', 'commit.gpgsign', 'false')
            hook.write_text('#!/bin/sh\nsleep 1\nexit 0\n')
            app.keys('<enter>')
            assert app.state()['workspace_busy']
            app.keys('q <space>')
            assert app.state()['workspace_busy']  # cannot exit or change index mid-commit
            app.wait(lambda: not app.state()['workspace_busy'])
            assert git(root, 'log', '-1', '--format=%s').stdout.strip() == 'retained'
            assert git(root, 'show', 'HEAD:a.txt').stdout == 'worktree-only\n'
            assert 'retained' in app.screen()
            app.keys('2 G')
            app.loaded()
            assert app.state()['workspace_file'] == 'z59.txt'
            assert 'untracked 59' in app.screen()
            app.keys('4 G <enter>')
            app.loaded()
            assert app.state()['workspace_commit'] == original
            before = git(root, 'diff', '--cached').stdout
            app.keys('<space>')
            assert git(root, 'diff', '--cached').stdout == before
            assert 'read-only' in app.screen()
            app.keys('0')
            assert 'Diff' in app.screen(54, 20)
            if os.environ.get('LAZYKEIFU_TEST_ARTIFACTS'):
                (evidence / 'workspace-narrow.txt').write_text(app.screen(54, 20))
            assert 'Diff' in app.screen(20, 6)
            app.keys('4')
            assert 'initial' in app.screen(54, 20)
            app.keys('2 a')
            app.loaded()
            assert len(git(root, 'diff', '--cached', '--name-only').stdout.splitlines()) == 60
            app.keys('u')
            app.loaded()
            assert git(root, 'diff', '--cached', '--quiet', check=False).returncode == 0
            assert (root / 'z59.txt').exists()
            app.keys('2 g j j k')
            app.loaded()
            assert app.state()['workspace_file'] == 'z01.txt'
            assert 'untracked 1' in app.screen()
            app.keys('?')
            assert 'Stash and hunk staging' in app.screen()
            assert 'p: pull' in app.screen()
            app.keys('<esc>')
            app.keys('q')
            app.proc.wait(timeout=3)
            assert app.proc.returncode == 0
        finally:
            app.close()
        print('PASS: partial index, 61 files, hook failure/retry, mutation lock, commit graph, history read-only, narrow layouts, rapid selection')

        # A linked worktree must operate on its own index and HEAD.
        worktree = Path(temp) / 'linked'
        git(root, 'worktree', 'add', '-b', 'linked', str(worktree))
        (worktree / 'linked.txt').write_text('linked change\n')
        app = Tui(worktree, config)
        try:
            app.loaded()
            assert app.state()['head'] == 'linked'
            assert app.state()['workspace_file'] == 'linked.txt'
            app.keys('<space> c l i n k e d <enter>')
            app.wait(lambda: not app.state()['workspace_busy'])
            assert git(worktree, 'log', '-1', '--format=%s').stdout.strip() == 'linked'
            assert git(root, 'log', '-1', '--format=%s').stdout.strip() == 'retained'
        finally:
            app.close()
        print('PASS: linked worktree index and HEAD isolation')

        unborn = Path(temp) / 'unborn'
        unborn.mkdir()
        init(unborn)
        app = Tui(unborn, config)
        try:
            app.loaded()
            app.keys('4')
            assert 'Commits' in app.screen()
            (unborn / 'first.txt').write_text('first\n')
            app.keys('R 2')
            app.loaded()
            app.keys('<space> c f i r s t <enter>')
            app.wait(lambda: not app.state()['workspace_busy'])
            assert git(unborn, 'log', '-1', '--format=%s').stdout.strip() == 'first'
            app.keys('4 <enter>')
            app.loaded()
            assert '+first' in app.screen(), (app.state(), app.screen())
        finally:
            app.close()
        print('PASS: initial commit and root-commit diff')


if __name__ == '__main__':
    run()

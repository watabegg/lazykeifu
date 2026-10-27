#!/usr/bin/env python3
"""Real TUI regression: file context, complete stats, refresh and graph checkout."""
import os
import runpy
import tempfile
import time
from pathlib import Path

w = runpy.run_path(str(Path(__file__).with_name('test-workspace.py')))
Tui, git, init = (w[n] for n in ('Tui', 'git', 'init'))


def run():
    with tempfile.TemporaryDirectory(prefix='lazykeifu-file-context-') as temp:
        base = Path(temp)
        root = base / 'repo'
        root.mkdir()
        hooks = init(root)
        for i in range(55):
            (root / f'file{i:02}.txt').write_text('old\n')
        (root / 'binary.dat').write_bytes(b'\0old')
        git(root, 'add', '.')
        git(root, 'commit', '-m', 'base')
        original = git(root, 'rev-parse', 'HEAD').stdout.strip()
        for i in range(55):
            (root / f'file{i:02}.txt').write_text('new\n')
        (root / 'binary.dat').write_bytes(b'\0new')
        git(root, 'commit', '-am', 'change all files')
        head = git(root, 'rev-parse', 'HEAD').stdout.strip()
        git(root, 'branch', 'topic')
        remote = base / 'remote.git'
        git(base, 'init', '--bare', '-b', 'main', str(remote))
        git(root, 'remote', 'add', 'origin', str(remote))
        git(root, 'push', '-u', 'origin', 'main')
        (hooks / 'pre-push').write_text('#!/bin/sh\nsleep 0.5\n')
        (hooks / 'pre-push').chmod(0o755)
        config = base / 'config'
        (config / 'lazykeifu').mkdir(parents=True)
        # Exercise the default local refresh interval; leave network polling off.
        (config / 'lazykeifu/config.toml').write_text('[refresh]\nauto_fetch=false\n')
        evidence = Path(os.environ.get('LAZYKEIFU_TEST_ARTIFACTS', str(base / 'evidence')))
        evidence.mkdir(parents=True, exist_ok=True)
        tui = Tui(root, config, stdin_pipe=True)
        try:
            tui.keys('4 <home>')
            tui.loaded()
            screen = tui.screen()
            assert f'Commit {head[:7]}' in screen and '+55' in screen and '-55' in screen, screen
            assert 'binary' in next(line for line in screen.splitlines() if 'binary.dat' in line), screen
            (evidence / 'commit-files.txt').write_text(screen)
            tui.keys('6 <end>')
            assert 'Changed files' not in tui.screen(), tui.screen()
            tui.keys('4 <enter> <end>')
            tui.loaded()
            screen = tui.screen()
            last = next(line for line in screen.splitlines() if 'file54.txt' in line)
            assert '+1' in last and '-1' in last, screen
            # Click the last visible file using its actual screen row.
            y = next(i for i, line in enumerate(screen.splitlines()) if 'file53.txt' in line)
            tui.request(cmd='mouse', kind='click', x=10, y=y)
            assert tui.state()['workspace_file'] == 'file53.txt', tui.screen()
            tui.keys('<esc> 4 <home> P <enter>')
            tui.wait(lambda: tui.state()['is_pushing'])
            tui.wait(lambda: not tui.state()['is_pushing'])
            tui.loaded()
            assert tui.state()['workspace_commit'] == head
            assert f'Commit {head[:7]}' in tui.screen()
            assert not git(root, 'status', '--porcelain').stdout
            # A regular Space selects a branch and never silently detaches HEAD.
            tui.keys('<space>')
            assert tui.state()['mode'] == 'git_menu', tui.screen()
            assert set(tui.state()['git_menu']['items']) == {'main', 'topic'}
            tui.keys('<end> <enter>')
            tui.wait(lambda: not tui.state()['workspace_busy'])
            assert git(root, 'symbolic-ref', '--short', 'HEAD').stdout.strip() == 'topic'
            tui.keys('4 <end> <space>')
            assert tui.state()['mode'] == 'normal', tui.screen()
            assert git(root, 'symbolic-ref', '--short', 'HEAD').stdout.strip() == 'topic'
            # Exercise the real terminal parser too: the plain o key detaches at the selected commit.
            tui.proc.stdin.write(b'o')
            tui.proc.stdin.flush()
            tui.wait(lambda: git(root, 'symbolic-ref', '-q', 'HEAD', check=False).returncode != 0)
            tui.wait(lambda: not tui.state()['workspace_busy'])
            assert git(root, 'rev-parse', 'HEAD').stdout.strip() == original
            assert 'detached HEAD' in tui.screen()
            # A remote-only label uses the existing tracking-branch checkout flow.
            git(root, 'update-ref', 'refs/remotes/origin/remote-only', original)
            tui.keys('2 r 4 <end> <space>')
            tui.wait(lambda: not tui.state()['workspace_busy'])
            assert git(root, 'symbolic-ref', '--short', 'HEAD').stdout.strip() == 'remote-only'
            assert git(root, 'rev-parse', '--abbrev-ref', '@{upstream}').stdout.strip() == 'origin/remote-only'
            # The branch picker still leads through the existing dirty-checkout decision.
            (root / 'file00.txt').write_text('keep dirty edit\n')
            tui.keys('2 r 4 <home> j <space>')
            assert tui.state()['mode'] == 'git_menu', tui.screen()
            tui.keys('<home> <enter>')
            assert tui.state()['git_menu']['items'][0] == 'Cancel', tui.screen()
            tui.keys('<esc>')
            assert (root / 'file00.txt').read_text() == 'keep dirty edit\n'
            assert git(root, 'symbolic-ref', '--short', 'HEAD').stdout.strip() == 'remote-only'
            git(root, 'restore', 'file00.txt')
            # External commit/push: the worktree list and header must clear promptly.
            git(root, 'switch', 'main')
            tui.keys('2 r')
            (root / 'file00.txt').write_text('external edit\n')
            tui.wait(lambda: tui.state()['workspace_file_count'] == 1)
            tui.loaded()
            assert 'Uncommitted' in tui.screen(), tui.screen()
            (evidence / 'uncommitted-files.txt').write_text(tui.screen())
            start = time.monotonic()
            git(root, 'commit', '-am', 'external commit')
            git(root, 'push')
            tui.wait(lambda: tui.state()['workspace_file_count'] == 0)
            elapsed = time.monotonic() - start
            assert elapsed < 3, elapsed
            assert 'Clean working tree' in tui.screen(), tui.screen()
            print(f'PASS: commit/worktree context, 56-file stats, mouse, push, branch picker, branchless Space, terminal o; external refresh {elapsed:.2f}s')
        finally:
            tui.close()
        # Linked worktrees have their own index and HEAD; refresh must use those.
        linked = base / 'linked'
        git(root, 'worktree', 'add', '-b', 'linked', str(linked))
        tui = Tui(linked, config)
        try:
            tui.keys('2')
            (linked / 'file00.txt').write_text('linked edit\n')
            tui.wait(lambda: tui.state()['workspace_file_count'] == 1)
            git(linked, 'commit', '-am', 'linked commit')
            start = time.monotonic()
            tui.wait(lambda: tui.state()['workspace_file_count'] == 0)
            assert time.monotonic() - start < 3
            assert 'Clean working tree' in tui.screen(), tui.screen()
            print('PASS: external commit refresh in linked worktree')
        finally:
            tui.close()


if __name__ == '__main__':
    run()

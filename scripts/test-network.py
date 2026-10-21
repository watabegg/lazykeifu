#!/usr/bin/env python3
"""Drive sync dialogs and conflict recovery against local remotes, without network access."""
import runpy
import tempfile
from pathlib import Path

workspace = runpy.run_path(str(Path(__file__).with_name('test-workspace.py')))
Tui, git, init = (workspace[name] for name in ('Tui', 'git', 'init'))


def run():
    with tempfile.TemporaryDirectory(prefix='lazykeifu-network-') as temp:
        base = Path(temp)
        root = base / 'local'
        root.mkdir()
        init(root)
        remote = base / 'team.git'
        git(base, 'init', '--bare', '-b', 'main', str(remote))
        (root / 'file').write_text('base\n')
        git(root, 'add', 'file')
        git(root, 'commit', '-m', 'base')
        git(root, 'remote', 'add', 'team', str(remote))
        git(root, 'push', '-u', 'team', 'main')
        peer = base / 'peer'
        git(base, 'clone', str(remote), str(peer))
        for repo in (root, peer):
            git(repo, 'config', 'user.name', 'Test')
            git(repo, 'config', 'user.email', 'test@example.invalid')
            git(repo, 'config', 'commit.gpgsign', 'false')
            git(repo, 'config', 'pull.ff', 'only')
        config = base / 'config'
        (config / 'lazykeifu').mkdir(parents=True)
        (config / 'lazykeifu/config.toml').write_text('[refresh]\nauto_fetch=false\nauto_refresh=false\n')
        tui = Tui(root, config)
        try:
            assert 'main' in tui.screen()
            (peer / 'file').write_text('remote update\n')
            git(peer, 'commit', '-am', 'remote-update')
            git(peer, 'push')
            old = git(root, 'rev-parse', 'HEAD').stdout
            tui.keys('4 <end> p')
            assert "Pull current branch 'main'?" in tui.screen()
            tui.keys('<esc>')
            assert git(root, 'rev-parse', 'HEAD').stdout == old
            tui.keys('p <enter>')
            tui.wait(lambda: not tui.state()['workspace_busy'])
            tui.loaded()
            assert git(root, 'rev-parse', 'HEAD').stdout == git(peer, 'rev-parse', 'HEAD').stdout
            assert 'remote-update' in tui.screen()

            # Verify non-origin push, and keep mutations blocked while the hook runs.
            (root / 'file').write_text('local update\n')
            git(root, 'commit', '-am', 'local-update')
            hook = root / '.git' / 'test-hooks' / 'pre-push'
            hook.write_text('#!/bin/sh\nsleep 1\n')
            hook.chmod(0o755)
            tui.keys('2 R 4 <end> P')
            assert "Push current branch 'main'?" in tui.screen()
            tui.keys('<enter>')
            tui.wait(lambda: tui.state()['is_pushing'])
            tui.keys('b c p')
            assert tui.state()['mode'] == 'normal'
            tui.wait(lambda: not tui.state()['is_pushing'])
            assert git(remote, 'rev-parse', 'main').stdout == git(root, 'rev-parse', 'HEAD').stdout
            assert 'Push completed' in tui.screen()

            git(peer, 'pull')
            (peer / 'file').write_text('conflicting remote\n')
            git(peer, 'commit', '-am', 'conflicting-remote')
            git(peer, 'push')
            (root / 'file').write_text('conflicting local\n')
            git(root, 'commit', '-am', 'conflicting-local')
            git(root, 'config', 'pull.ff', 'true')
            git(root, 'config', 'pull.rebase', 'false')
            tui.keys('2 R 4 p <enter>')
            tui.wait(lambda: not tui.state()['workspace_busy'])
            assert tui.state()['mode'] == 'error'
            assert 'CONFLICT' in tui.screen()
            tui.keys('<esc> 2')
            tui.loaded()
            screen = tui.screen()
            assert 'MERGING' in screen and 'file' in screen
            assert git(root, 'ls-files', '-u').stdout
            git(root, 'merge', '--abort')
            tui.keys('R')
            assert 'MERGING' not in tui.screen()
            print('PASS: pull confirmation/cancel, non-origin pull/push, push lock, conflict visibility and external abort refresh')
        finally:
            tui.close()


if __name__ == '__main__':
    run()

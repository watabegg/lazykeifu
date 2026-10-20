#!/usr/bin/env python3
"""Pane tabs, Git porcelain statuses, and opening worktrees/submodules."""
import os
import runpy
import tempfile
from pathlib import Path
w = runpy.run_path(str(Path(__file__).with_name('test-workspace.py')))
Tui, git, init = (w[n] for n in ('Tui', 'git', 'init'))


def run():
    with tempfile.TemporaryDirectory(prefix='pedigit-tabs-') as temp:
        base = Path(temp)
        root = base / 'project'
        root.mkdir()
        init(root)
        for name in ('modified', 'partial', 'deleted', 'renamed'):
            (root / name).write_text('base\n')
        git(root, 'add', '.')
        git(root, 'commit', '-m', 'base')
        git(root, 'tag', 'v1.0')
        git(root, 'branch', 'topic')
        git(root, 'update-ref', 'refs/remotes/team/topic', 'HEAD')
        linked = base / 'linked worktree'
        git(root, 'worktree', 'add', '-b', 'linked', str(linked))
        module = base / 'module-source'
        module.mkdir()
        init(module)
        (module / 'module-file').write_text('module\n')
        git(module, 'add', '.')
        git(module, 'commit', '-m', 'module base')
        git(root, '-c', 'protocol.file.allow=always', 'submodule', 'add', str(module), 'modules/core')
        git(root, 'commit', '-am', 'add submodule')
        (root / 'modified').write_text('modified\n')
        (root / 'partial').write_text('staged\n')
        git(root, 'add', 'partial')
        (root / 'partial').write_text('unstaged\n')
        (root / 'deleted').unlink()
        git(root, 'mv', 'renamed', 'new-name')
        (root / 'added').write_text('new staged\n')
        git(root, 'add', 'added')
        (root / 'unknown').write_text('untracked\n')
        config = base / 'config'
        (config / 'pedigit').mkdir(parents=True)
        (config / 'pedigit/config.toml').write_text('[refresh]\nauto_fetch=false\nauto_refresh=false\n')
        tui = Tui(root, config)
        try:
            tui.loaded()
            screen = tui.screen()
            assert tui.state()['layout']['files'][2] == 48
            assert '[Files] Worktrees  Submodules' in screen, screen
            for text in ('A  added', ' D deleted', ' M modified', 'MM partial', 'R  new-name', '?? unknown'):
                assert text in screen, (text, screen)
            original = git(root, 'write-tree').stdout
            tui.keys('2 ]')
            assert tui.state()['files_tab'] == 'Worktrees'
            screen = tui.screen()
            assert '[Worktrees]' in screen and 'linked' in screen, screen
            tui.keys('a c d')
            assert tui.state()['mode'] == 'normal'
            assert git(root, 'write-tree').stdout == original
            tui.keys(']')
            assert tui.state()['files_tab'] == 'Submodules'
            assert 'modules/core' in tui.screen()
            tui.keys('[')
            # Click Files on the tab header.
            screen = tui.screen()
            x, y, _, _ = tui.state()['layout']['files']
            tui.request(cmd='mouse', kind='click', x=x+5, y=y)
            assert tui.state()['files_tab'] == 'Files'
            # Every visible tab label maps to its actual header cells.
            for label, expected in [('Submodules', 'Submodules'), ('Worktrees', 'Worktrees'), ('Files', 'Files')]:
                screen = tui.screen()
                x, y, _, _ = tui.state()['layout']['files']
                column = screen.splitlines()[y].index(label)
                tui.request(cmd='mouse', kind='click', x=column+len(label)-1, y=y)
                assert tui.state()['files_tab'] == expected
            tui.keys('3')
            screen = tui.screen()
            assert '[Local] Remotes  Tags' in screen, screen
            # Remote branches are separated from local branches.
            tui.keys(']')
            assert 'team/topic' in tui.screen()
            assert tui.state()['refs_tab'] == 'Remotes'
            tui.keys(']')
            assert tui.state()['refs_tab'] == 'Tags'
            assert 'v1.0' in tui.screen()
            tui.keys('2 3')
            assert tui.state()['workspace_commit'] == git(root, 'rev-parse', 'v1.0').stdout.strip()
            tui.keys('<enter>')
            tui.loaded()
            assert tui.state()['workspace_commit'] == git(root, 'rev-parse', 'v1.0').stdout.strip()
            assert 'Diff' in tui.screen()
            assert 'read-only' in tui.screen()
            assert git(root, 'write-tree').stdout == original
            tui.keys('<esc> 2 ] <home> j <enter>')
            tui.loaded()
            assert Path(tui.state()['repo_path']).resolve() == linked.resolve(), tui.state()
            assert 'linked' in tui.screen().splitlines()[0]
            assert not tui.state()['workspace_diff_visible']
            tui.keys('2 ] <home> <enter>')
            tui.loaded()
            assert Path(tui.state()['repo_path']).resolve() == root.resolve()
            tui.keys('2 ] ] <enter>')
            tui.loaded()
            assert Path(tui.state()['repo_path']).resolve() == (root / 'modules/core').resolve()
        finally:
            tui.close()
        # Uninitialized modules stay visible, and Enter cannot open the parent by mistake.
        git(root, 'submodule', 'deinit', '-f', '--', 'modules/core')
        tui = Tui(root, config)
        try:
            tui.keys('2 ] ]')
            assert 'not initialized' in tui.screen()
            tui.keys('<enter>')
            assert tui.state()['mode'] == 'error'
            assert Path(tui.state()['repo_path']).resolve() == root.resolve()
            tui.keys('<esc> [ [')
            tui.loaded()
            if os.environ.get('PEDIGIT_TEST_ARTIFACTS'):
                dest = Path(os.environ['PEDIGIT_TEST_ARTIFACTS'])
                dest.mkdir(parents=True, exist_ok=True)
                (dest / 'tabs-wide.txt').write_text(tui.screen())
                (dest / 'tabs-narrow.txt').write_text(tui.screen(54, 20))
        finally:
            tui.close()
    print('PASS: pane tabs, XY status, filtered refs, tag diff, worktree/submodule opening, uninitialized module, mouse tabs')


if __name__ == '__main__':
    run()

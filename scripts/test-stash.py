#!/usr/bin/env python3
"""Exercise stash save, selection, previews, and destructive confirmations in the real TUI."""
import runpy
import tempfile
from pathlib import Path

workspace = runpy.run_path(str(Path(__file__).with_name('test-workspace.py')))
Tui, git, init = (workspace[name] for name in ('Tui', 'git', 'init'))


def run():
    with tempfile.TemporaryDirectory(prefix='lazykeifu-stash-') as temp:
        base = Path(temp)
        root = base / 'repo'
        root.mkdir()
        init(root)
        (root / 'file').write_text('base\n')
        git(root, 'add', 'file')
        git(root, 'commit', '-m', 'base')
        config = base / 'config'
        (config / 'lazykeifu').mkdir(parents=True)
        (config / 'lazykeifu/config.toml').write_text('[refresh]\nauto_fetch=false\nauto_refresh=false\n')
        tui = Tui(root, config)
        try:
            tui.keys('5')
            tui.loaded()
            assert 'No stashes' in tui.screen()
            (root / 'file').write_text('saved tracked\n')
            (root / 'untracked').write_text('saved untracked\n')
            tui.keys('R s')
            assert 'includes untracked' in tui.screen()
            tui.keys('w i p <enter>')
            tui.wait(lambda: not tui.state()['workspace_busy'])
            tui.loaded()
            assert tui.state()['workspace_stash_count'] == 1
            assert (root / 'file').read_text() == 'base\n'
            assert not (root / 'untracked').exists()
            screen = tui.screen()
            assert '5 Stashes' in screen and 'stash@{0}' in screen and 'wip' in screen
            assert '+saved tracked' in screen and '+saved untracked' in screen
            header = next(i for i, line in enumerate(screen.splitlines()) if '5 Stashes' in line)
            tui.request(cmd='mouse', kind='click', x=5, y=header)
            tui.loaded()
            assert '+saved tracked' in tui.screen()
            tui.request(cmd='mouse', kind='scroll_down', x=5, y=5)
            tui.loaded()
            assert tui.state()['workspace_stash'] is None
            tui.keys('5')
            tui.loaded()
            assert '5 Stashes' in tui.screen(54, 20)
            tui.keys('<enter>')
            assert tui.state()['workspace_focus'] == 'Diff'
            assert 'stash ' in tui.screen(54, 20)
            before = git(root, 'status', '--porcelain').stdout
            tui.keys('<space>')
            assert git(root, 'status', '--porcelain').stdout == before
            tui.keys('5 <space>')
            assert 'Apply stash@{0}' in tui.screen()
            tui.keys('<esc>')
            assert (root / 'file').read_text() == 'base\n'
            tui.keys('<space> <enter>')
            tui.wait(lambda: not tui.state()['workspace_busy'])
            assert (root / 'file').read_text() == 'saved tracked\n'
            assert (root / 'untracked').read_text() == 'saved untracked\n'
            assert len(git(root, 'stash', 'list').stdout.splitlines()) == 1
            # Drop affects only the stash; worktree remains as applied.
            tui.keys('d')
            assert 'Drop stash@{0}' in tui.screen()
            tui.keys('<esc>')
            assert len(git(root, 'stash', 'list').stdout.splitlines()) == 1
            tui.keys('d <enter>')
            tui.wait(lambda: not tui.state()['workspace_busy'])
            tui.loaded()
            assert tui.state()['workspace_stash_count'] == 0
            assert (root / 'untracked').exists()
            assert 'No stashes' in tui.screen()
            # Save again and pop: both tracked and untracked content restored.
            tui.keys('s p o p <enter>')
            tui.wait(lambda: not tui.state()['workspace_busy'])
            tui.keys('p')
            assert 'Pop stash@{0}' in tui.screen()
            tui.keys('<enter>')
            tui.wait(lambda: not tui.state()['workspace_busy'])
            assert not git(root, 'stash', 'list').stdout
            assert (root / 'untracked').exists()
            # A stash pushed outside the app must not change a confirmed drop target.
            tui.keys('s o l d <enter>')
            tui.wait(lambda: not tui.state()['workspace_busy'])
            tui.keys('d')
            assert 'Drop stash@{0}' in tui.screen()
            (root / 'file').write_text('external stash\n')
            git(root, 'stash', 'push', '-m', 'external')
            tui.keys('<enter>')
            tui.wait(lambda: not tui.state()['workspace_busy'])
            assert tui.state()['mode'] == 'error'
            assert 'Stash list changed' in tui.screen()
            assert len(git(root, 'stash', 'list').stdout.splitlines()) == 2
            tui.keys('<esc> 5 g')
            tui.loaded()
            assert '+external stash' in tui.screen()
            tui.keys('j')
            tui.loaded()
            assert '+saved untracked' in tui.screen()
            screen = tui.screen()
            row = next(i for i, line in enumerate(screen.splitlines()) if 'stash@{0}' in line)
            tui.request(cmd='mouse', kind='click', x=5, y=row)
            tui.loaded()
            assert '+external stash' in tui.screen()
            tui.keys('j')
            tui.loaded()
            # Conflicting pop keeps the selected stash and exposes the conflict files.
            (root / 'file').write_text('new base\n')
            git(root, 'commit', '-am', 'new base')
            tui.keys('p <enter>')
            tui.wait(lambda: not tui.state()['workspace_busy'])
            assert tui.state()['mode'] == 'error'
            assert 'CONFLICT' in tui.screen()
            assert len(git(root, 'stash', 'list').stdout.splitlines()) == 2
            tui.keys('<esc> 2')
            tui.loaded()
            screen = tui.screen()
            assert '!  file' in screen or '!* file' in screen, screen
            assert git(root, 'ls-files', '-u').stdout
            print('PASS: stash save including untracked, preview, narrow layout, apply/pop/drop, cancel, stale selection, conflict retention')
        finally:
            tui.close()


if __name__ == '__main__':
    run()

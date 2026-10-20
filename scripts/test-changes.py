#!/usr/bin/env python3
"""Real TUI tests for partial index operations and confirmed unstaged discard."""
import runpy
import tempfile
from pathlib import Path

workspace = runpy.run_path(str(Path(__file__).with_name('test-workspace.py')))
Tui, git, init = (workspace[name] for name in ('Tui', 'git', 'init'))


def run():
    with tempfile.TemporaryDirectory(prefix='lazykeifu-changes-') as temp:
        base = Path(temp)
        root = base / 'repo'
        root.mkdir()
        init(root)
        original = ''.join(f'line {i}\n' for i in range(40))
        changed = original.replace('line 2\n', 'first edit\n').replace('line 30\n', 'second edit\n')
        file = root / '[x] file.txt'
        file.write_text(original)
        git(root, 'add', '.')
        git(root, 'commit', '-m', 'base')
        file.write_text(changed)
        config = base / 'config'
        (config / 'lazykeifu').mkdir(parents=True)
        (config / 'lazykeifu/config.toml').write_text('[refresh]\nauto_fetch=false\nauto_refresh=false\n')
        tui = Tui(root, config)
        try:
            tui.loaded()
            tui.keys('<enter>')
            assert 'hunk 1/2' in tui.screen()
            tui.keys('l')
            assert 'hunk 2/2' in tui.screen() and '+second edit' in tui.screen()
            assert 'hunk 2/2' in tui.screen(54, 20)
            tui.screen()
            tui.keys('<space>')
            tui.wait(lambda: not tui.state()['workspace_busy'])
            tui.loaded()
            assert git(root, 'show', ':[x] file.txt').stdout == original.replace('line 30\n', 'second edit\n')
            assert file.read_text() == changed
            assert 'hunk 1/1' in tui.screen()
            tui.keys('<tab>')
            tui.loaded()
            assert 'STAGED' in tui.screen() and '+second edit' in tui.screen()
            tui.keys('<space>')
            tui.wait(lambda: not tui.state()['workspace_busy'])
            tui.loaded()
            assert git(root, 'show', ':[x] file.txt').stdout == original
            assert file.read_text() == changed
            # External edit after the diff was displayed must not stage stale content.
            file.write_text('external change\n')
            tui.keys('<space>')
            tui.wait(lambda: not tui.state()['workspace_busy'])
            assert tui.state()['mode'] == 'error'
            assert 'Diff changed' in tui.screen()
            assert git(root, 'show', ':[x] file.txt').stdout == original
            tui.keys('<esc>')
            # Partial index state: discard must restore exactly the index, not HEAD.
            file.write_text('staged version\n')
            git(root, 'add', '--', '[x] file.txt')
            file.write_text('unstaged version\n')
            tui.keys('R 2')
            tui.loaded()
            tui.keys('d')
            assert 'Staged content is kept' in tui.screen()
            tui.keys('<esc>')
            assert file.read_text() == 'unstaged version\n'
            tui.keys('d')
            file.write_text('edit while confirming\n')
            tui.keys('<enter>')
            tui.wait(lambda: not tui.state()['workspace_busy'])
            assert tui.state()['mode'] == 'error'
            assert 'File or index changed' in tui.screen()
            assert file.read_text() == 'edit while confirming\n'
            tui.keys('<esc> R 2')
            tui.loaded()
            tui.keys('d <enter>')
            tui.wait(lambda: not tui.state()['workspace_busy'])
            tui.loaded()
            assert file.read_text() == 'staged version\n'
            assert git(root, 'show', ':[x] file.txt').stdout == 'staged version\n'
            # Staged-only data cannot be discarded implicitly.
            tui.keys('d')
            assert tui.state()['mode'] == 'normal'
            assert 'Switch to the unstaged diff' in tui.screen()
            assert file.read_text() == 'staged version\n'
            # Literal untracked path deletion requires confirmation; cancel is harmless.
            (root / 'z[x]').write_text('remove\n')
            (root / 'zx').write_text('keep\n')
            tui.keys('R 2 j')
            tui.loaded()
            assert tui.state()['workspace_file'] == 'z[x]'
            tui.keys('d')
            assert 'Delete untracked file z[x] permanently?' in tui.screen()
            tui.keys('<esc>')
            assert (root / 'z[x]').exists()
            tui.keys('d <enter>')
            tui.wait(lambda: not tui.state()['workspace_busy'])
            assert not (root / 'z[x]').exists()
            assert (root / 'zx').exists()
            # Historical diff hunk remains read-only.
            tui.keys('4 <end> <enter> <enter>')
            tui.loaded()
            index = git(root, 'write-tree').stdout
            tui.keys('<space>')
            assert 'read-only' in tui.screen()
            assert git(root, 'write-tree').stdout == index
            print('PASS: hunk selection/stage/unstage, stale diff refusal, discard/cancel/index retention, stale confirmation, literal untracked deletion, history read-only')
        finally:
            tui.close()


if __name__ == '__main__':
    run()

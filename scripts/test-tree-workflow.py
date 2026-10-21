#!/usr/bin/env python3
"""Exercise collapsible files, contextual keys and stage-all commit confirmation."""
import runpy
import tempfile
from pathlib import Path
w = runpy.run_path(str(Path(__file__).with_name('test-workspace.py')))
Tui, git, init = (w[n] for n in ('Tui', 'git', 'init'))


def run():
    with tempfile.TemporaryDirectory(prefix='lazykeifu-tree-') as temp:
        base = Path(temp); root = base / 'project'; root.mkdir(); init(root)
        for name in ('src/a', 'src/nested/b', 'z'):
            path = root / name; path.parent.mkdir(parents=True, exist_ok=True); path.write_text('base\n')
        git(root, 'add', '.'); git(root, 'commit', '-m', 'base')
        for name in ('src/a', 'src/nested/b', 'z'):
            (root / name).write_text('changed-' + name + '\n')
        (root / 'new').write_text('untracked\n')
        config = base / 'config'; (config / 'lazykeifu').mkdir(parents=True)
        (config / 'lazykeifu/config.toml').write_text('[refresh]\nauto_fetch=false\nauto_refresh=false\n')
        tui = Tui(root, config)
        try:
            tui.keys('2 <home> j'); tui.loaded()
            screen = tui.screen()
            assert '▾ src/' in screen and 'nested/' in screen, screen
            assert tui.state()['workspace_file'] is None
            assert 'fold' in screen.splitlines()[-2]
            tui.keys('<enter>'); screen = tui.screen()
            assert '▸ src/' in screen and 'nested/' not in '\n'.join(line[:tui.state()['layout']['files'][2]] for line in screen.splitlines()), screen
            tui.keys('R'); assert '▸ src/' in tui.screen()
            assert not git(root, 'diff', '--cached', '--name-only').stdout.strip()
            tui.keys('<enter> j'); tui.loaded()
            assert tui.state()['workspace_file'] == 'src/a', tui.state()
            assert 'Space stage' in tui.screen().splitlines()[-2]
            tui.keys('<space>'); tui.loaded()
            assert git(root, 'diff', '--cached', '--name-only').stdout.strip() == 'src/a'
            tui.keys('<enter>'); tui.loaded()
            assert '+changed-src/a' in tui.screen()
            assert 'unstage hunk' in tui.screen().splitlines()[-2]
            tui.keys('<esc> k <enter>')
            assert 'nested/' not in '\n'.join(line[:tui.state()['layout']['files'][2]] for line in tui.screen().splitlines())
            assert git(root, 'diff', '--cached', '--name-only').stdout.strip() == 'src/a'
            # Clicking a folder expands it without selecting/staging an unrelated file.
            screen = tui.screen(); y = next(i for i, line in enumerate(screen.splitlines()) if '▸ src/' in line)
            tui.request(cmd='mouse', kind='click', x=8, y=y)
            assert 'nested/' in tui.screen()
            assert tui.state()['workspace_file'] is None
            tui.keys('4'); assert 'revert' in tui.screen().splitlines()[-2]
            tui.keys('<left>'); assert tui.state()['workspace_focus'] == 'Stashes'
            tui.keys('2 a a'); tui.loaded()
            assert not git(root, 'diff', '--cached', '--name-only').stdout.strip()
            tui.screen(); layout = tui.state()['layout']
            tui.keys('c'); screen = tui.screen()
            assert tui.state()['mode'] == 'confirm' and 'Nothing staged' in screen, screen
            assert 'Enter / y' in screen
            tui.keys('<esc>')
            assert not git(root, 'diff', '--cached', '--name-only').stdout.strip()
            tui.keys('c <enter>'); tui.loaded(); screen = tui.screen()
            assert tui.state()['mode'] == 'input', screen
            assert tui.state()['layout'] == layout  # dialog overlays the unchanged workspace
            assert screen.splitlines()[2].index('╭ Commit') == 4
            assert 'STAGED diff' in screen and 'Commit · main' in screen and '4 Graph' in screen
            assert set(git(root, 'diff', '--cached', '--name-only').stdout.splitlines()) == {'new', 'src/a', 'src/nested/b', 'z'}
            tui.keys('t r e e <enter>'); tui.wait(lambda: not tui.state()['workspace_busy'])
            assert tui.state()['mode'] == 'normal'
            assert git(root, 'log', '-1', '--format=%s').stdout.strip() == 'tree'
            assert not git(root, 'status', '--porcelain').stdout.strip()
        finally:
            tui.close()
    print('PASS: folder fold/refresh/mouse, file stage/diff, contextual keys, cancel stage-all, accept and commit untracked + tracked files')


if __name__ == '__main__':
    run()

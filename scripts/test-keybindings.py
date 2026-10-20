#!/usr/bin/env python3
"""Exercise contextual Lazygit shortcuts and aligned detail statistics in the real TUI."""
import runpy
import tempfile
from pathlib import Path

w = runpy.run_path(str(Path(__file__).with_name('test-workspace.py')))
Tui, git, init = (w[n] for n in ('Tui', 'git', 'init'))


def run():
    with tempfile.TemporaryDirectory(prefix='pedigit-bindings-') as temp:
        base = Path(temp)
        root = base / 'repo'
        root.mkdir()
        init(root)
        (root / 'small.txt').write_text('old\n')
        (root / 'large.txt').write_text('old\n' * 1000)
        (root / 'binary.dat').write_bytes(b'\x00old')
        git(root, 'add', '.')
        git(root, 'commit', '-m', 'base')
        (root / 'small.txt').write_text('new\n')
        (root / 'large.txt').write_text('new\n' * 125)
        (root / 'binary.dat').write_bytes(b'\x00new')
        git(root, 'commit', '-am', 'sized changes')
        head = git(root, 'rev-parse', 'HEAD').stdout.strip()
        config = base / 'config'
        (config / 'pedigit').mkdir(parents=True)
        (config / 'pedigit/config.toml').write_text('[refresh]\nauto_fetch=false\nauto_refresh=false\n')
        tui = Tui(root, config)
        try:
            tui.keys('4 <home>')
            tui.wait(lambda: '+126' in tui.screen() and '-1001' in tui.screen())
            tui.keys('6 <end>')
            screen = tui.screen()
            positions = []
            for name in ('small.txt', 'large.txt', 'binary.dat'):
                row = next(line for line in screen.splitlines() if name in line)
                positions.append(row.index(name))
            assert len(set(positions)) == 1, screen
            assert 'binary' in screen, screen
            # Actions operate on the focused pane; no global x action palette.
            tui.keys('4 x')
            assert tui.state()['mode'] == 'normal', tui.screen()
            tui.keys('@')
            assert 'Extras' in tui.screen()
            tui.keys('<esc> r')
            assert tui.state()['mode'] == 'confirm' and 'Edit message only' in tui.screen(), tui.screen()
            tui.keys('<esc> g')
            assert tui.state()['mode'] == 'git_menu' and '--soft' in tui.screen(), tui.screen()
            tui.keys('<esc> T')
            assert tui.state()['mode'] == 'input' and 'tag' in tui.screen(), tui.screen()
            tui.keys('<esc> t')
            assert tui.state()['mode'] == 'confirm' and 'revert' in tui.screen(), tui.screen()
            tui.keys('<esc> C j V')
            assert tui.state()['mode'] == 'confirm' and head[:7] in tui.screen(), tui.screen()
            assert git(root, 'rev-parse', 'HEAD').stdout.strip() == head
            tui.keys('<esc> <c-r> V')
            assert tui.state()['mode'] == 'error', tui.screen()
            tui.keys('<esc> <home> <space>')
            assert 'detached HEAD' in tui.screen(), tui.screen()
            tui.keys('<esc> 3 R')
            assert tui.state()['mode'] == 'input' and 'Rename' in tui.screen(), tui.screen()
            tui.keys('<esc> n')
            assert 'New branch from selected' in tui.screen(), tui.screen()
            tui.keys('<esc> <enter>')
            assert tui.state()['workspace_focus'] == 'Graph', tui.screen()
            tui.keys('2')
            (root / 'dir').mkdir()
            (root / 'dir/a').write_text('a\n')
            (root / 'dir/b').write_text('b\n')
            (root / 'dir-other').write_text('untouched\n')
            tui.keys('r <home>')
            tui.loaded()
            tui.keys('- <space>')
            staged = git(root, 'diff', '--cached', '--name-only').stdout.splitlines()
            assert staged == ['dir/a', 'dir/b'], (staged, tui.screen())
            tui.keys('<space>')
            assert not git(root, 'diff', '--cached', '--name-only').stdout
            tui.keys('a')
            assert len(git(root, 'diff', '--cached', '--name-only').stdout.splitlines()) == 3
            tui.keys('a')
            assert not git(root, 'diff', '--cached', '--name-only').stdout
            tui.keys('= <end> <enter>')
            tui.loaded()
            last_file = tui.state()['workspace_file']
            tui.keys('N')
            tui.loaded()
            assert tui.state()['workspace_file'] != last_file, tui.screen()
            tui.keys('n')
            tui.loaded()
            assert tui.state()['workspace_file'] == last_file, tui.screen()
            tui.keys('<tab>')
            tui.loaded()
            assert tui.state()['workspace_staged'], tui.screen()
            tui.keys('<tab> 2 A')
            assert tui.state()['mode'] == 'confirm' and 'staged changes' in tui.screen(), tui.screen()
            tui.keys('<esc> s w i p <enter>')
            tui.wait(lambda: not tui.state()['workspace_busy'])
            tui.keys('5 g')
            assert 'Pop stash' in tui.screen(), tui.screen()
            tui.keys('<esc> p')
            assert 'Pop stash' not in tui.screen(), tui.screen()
            assert git(root, 'stash', 'list').stdout
        finally:
            tui.close()
    print('PASS: aligned +/- and binary filenames, contextual amend/reword/reset/tag/revert, copy/paste selection, checkout/history, directory staging, stage-all toggle, stash pop')


if __name__ == '__main__':
    run()

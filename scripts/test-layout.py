#!/usr/bin/env python3
"""Graph-first workspace: on-demand diff, compact layout, focus and status."""
import os
import runpy
import tempfile
from pathlib import Path
w = runpy.run_path(str(Path(__file__).with_name('test-workspace.py')))
Tui, git, init = (w[name] for name in ('Tui', 'git', 'init'))


def run():
    with tempfile.TemporaryDirectory(prefix='lazykeifu-layout-') as temp:
        base = Path(temp)
        root = base / 'project'
        root.mkdir()
        init(root)
        for i in range(25):
            (root / 'file.txt').write_text(f'version {i}\n')
            git(root, 'add', '.')
            git(root, 'commit', '-m', f'Change {i:02}: graph remains the main view')
        git(root, 'branch', 'feature/long-branch-with-a-useful-description')
        config = base / 'config'
        (config / 'lazykeifu').mkdir(parents=True)
        (config / 'lazykeifu/config.toml').write_text('[refresh]\nauto_fetch=false\nauto_refresh=false\n')
        tui = Tui(root, config)
        try:
            tui.loaded()
            closed = tui.screen()
            assert 'clean' in closed.splitlines()[0]
            assert 'Clean working tree' in closed and 'No stashes' in closed
            assert '0 Diff' not in closed
            assert tui.state()['layout']['graph'][3] >= 27
            assert tui.state()['layout']['files'][3] >= 6
            assert tui.state()['layout']['files'][2] == 48
            # Moving through panels never reveals a hidden preview.
            tui.keys('4 <tab>')
            assert tui.state()['workspace_focus'] == 'Files'
            assert not tui.state()['workspace_diff_visible']
            tui.keys('4 j')
            selected = tui.state()['selected_commit']
            tui.keys('<enter>')
            tui.loaded()
            opened = tui.screen()
            assert '0 Diff' in opened and '4 Graph' in opened
            assert tui.state()['workspace_diff_visible']
            assert 'clean' in opened.splitlines()[0]  # historical file count isn't worktree status
            tui.keys('<esc>')
            assert not tui.state()['workspace_diff_visible']
            assert tui.state()['workspace_focus'] == 'Graph'
            assert tui.state()['selected_commit'] == selected
            assert '0 Diff' not in tui.screen()
            assert tui.state()['layout']['graph'][3] >= 27
            (root / 'file.txt').write_text('local edit\n')
            tui.keys('R 2 <enter>')
            tui.loaded()
            assert '+local edit' in tui.screen()
            assert '1 changed' in tui.screen().splitlines()[0]
            tui.keys('<esc>')
            assert tui.state()['workspace_focus'] == 'Files'
            assert not tui.state()['workspace_diff_visible']
            tui.keys('0')
            tui.loaded()
            for width, height in ((180, 40), (120, 32), (80, 24), (54, 20)):
                screen = tui.screen(width, height)
                assert '4 Graph' in screen and '0 Diff' in screen, screen
                if os.environ.get('LAZYKEIFU_TEST_ARTIFACTS'):
                    dest = Path(os.environ['LAZYKEIFU_TEST_ARTIFACTS'])
                    dest.mkdir(parents=True, exist_ok=True)
                    (dest / f'layout-diff-{width}.txt').write_text(screen)
            assert 'Terminal too small' in tui.screen(20, 6)
            tui.keys('<esc> 4')
            tui.screen()
            assert not tui.state()['workspace_diff_visible']
            # A real mouse double-click on a graph row also opens the preview.
            x, y, _, _ = tui.state()['layout']['graph']
            tui.request(cmd='mouse', kind='click', x=x+3, y=y+2)
            tui.request(cmd='mouse', kind='click', x=x+3, y=y+2)
            assert tui.state()['workspace_diff_visible']
            tui.keys('<esc> 2 ?')
            narrow_help = tui.screen(54, 20)
            assert 'Help' in narrow_help
            tui.keys(' '.join(['j'] * 80))
            assert 'Conflicted file' in tui.screen()
            tui.keys('<esc>')
            tui.keys('4')
            final = tui.screen()
            assert '0 Diff' not in final
            if os.environ.get('LAZYKEIFU_TEST_ARTIFACTS'):
                (dest / 'layout-graph.txt').write_text(final)
                (dest / 'layout-clean.txt').write_text(closed)
        finally:
            tui.close()
    print('PASS: graph always visible, diff open/close, selection, compact viewports, status, mouse, scrollable help')


if __name__ == '__main__':
    run()

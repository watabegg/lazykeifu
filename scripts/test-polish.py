#!/usr/bin/env python3
"""Stable operation panes, fixed tab hit targets, and structured help."""
import os
import runpy
import tempfile
from pathlib import Path
w = runpy.run_path(str(Path(__file__).with_name('test-workspace.py')))
Tui, git, init = (w[n] for n in ('Tui', 'git', 'init'))


def run():
    with tempfile.TemporaryDirectory(prefix='lazykeifu-polish-') as temp:
        base = Path(temp)
        root = base / 'project'
        root.mkdir()
        init(root)
        (root / 'file').write_text('base\n')
        git(root, 'add', '.')
        git(root, 'commit', '-m', 'base')
        config = base / 'config'
        (config / 'lazykeifu').mkdir(parents=True)
        (config / 'lazykeifu/config.toml').write_text('[refresh]\nauto_fetch=false\nauto_refresh=false\n')
        tui = Tui(root, config)
        try:
            tui.loaded()
            tui.keys('2')
            tui.screen()
            original = tui.state()['layout']
            (root / 'file').write_text('modified\n')
            tui.keys('R')
            tui.loaded()
            tui.screen()
            assert tui.state()['layout'] == original
            for n in range(40):
                (root / f'new-{n:02}').write_text('new\n')
            tui.keys('R')
            tui.loaded()
            tui.screen()
            assert tui.state()['layout'] == original
            # All tab cells, including padding/brackets, retain their destination
            # regardless of which tab is selected. No coordinate recalculation.
            for panel, rect_name, state_key, labels in [
                ('2', 'files', 'files_tab', ['Files', 'Worktrees', 'Submodules']),
                ('3', 'branches', 'refs_tab', ['Local', 'Remotes', 'Tags']),
            ]:
                tui.keys(panel)
                tui.screen()
                x, y, _, _ = tui.state()['layout'][rect_name]
                starts = [x+4]
                for label in labels[:-1]:
                    starts.append(starts[-1]+len(label)+2)
                for start_tab in range(3):
                    tui.screen()
                    tui.request(cmd='mouse', kind='click', x=starts[start_tab], y=y)
                    for target in (2, 0, 1):
                        for edge in (0, len(labels[target])+1):
                            tui.screen()
                            tui.request(cmd='mouse', kind='click', x=starts[target]+edge, y=y)
                            assert tui.state()[state_key] == labels[target], (target, edge, tui.screen())
                            tui.screen()
                            assert tui.state()['layout'] == original
                    tui.keys(']')
            tui.keys('2 [ ?')
            assert 'Navigation' in tui.screen()
            help_wide = tui.screen()
            help_narrow = tui.screen(54, 20)
            assert 'Navigation' in help_narrow and 'Tab / Shift-Tab' in help_narrow
            tui.keys(' '.join(['j'] * 180))
            assert 'Conflicted file' in tui.screen()
            tui.keys('<esc> 2')
            if os.environ.get('LAZYKEIFU_TEST_ARTIFACTS'):
                dest = Path(os.environ['LAZYKEIFU_TEST_ARTIFACTS'])
                dest.mkdir(parents=True, exist_ok=True)
                (dest / 'help-wide.txt').write_text(help_wide)
                (dest / 'help-narrow.txt').write_text(help_narrow)
                (dest / 'workspace-wide.txt').write_text(tui.screen())
        finally:
            tui.close()
    print('PASS: stable pane geometry, every tab edge under repeated selection, wide/narrow help and scrolling')


if __name__ == '__main__':
    run()

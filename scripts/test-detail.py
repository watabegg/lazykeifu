#!/usr/bin/env python3
"""Commit detail restoration and compact, outside-dismissable help."""
import os
import runpy
import tempfile
from pathlib import Path
w = runpy.run_path(str(Path(__file__).with_name('test-workspace.py')))
Tui, git, init = (w[n] for n in ('Tui', 'git', 'init'))


def run():
    with tempfile.TemporaryDirectory(prefix='lazykeifu-detail-') as temp:
        base = Path(temp)
        root = base / 'project'
        root.mkdir()
        init(root)
        (root / 'file').write_text('base\n')
        git(root, 'add', '.')
        git(root, 'commit', '-m', 'first subject')
        (root / 'file').write_text('next\n')
        git(root, 'commit', '-am', 'second subject\n\n' + '\n'.join(f'Body line {i:02}' for i in range(30)))
        head = git(root, 'rev-parse', 'HEAD').stdout.strip()
        config = base / 'config'
        (config / 'lazykeifu').mkdir(parents=True)
        (config / 'lazykeifu/config.toml').write_text('[refresh]\nauto_fetch=false\nauto_refresh=false\n')
        tui = Tui(root, config)
        try:
            tui.loaded()
            tui.keys('4 g')
            screen = tui.screen()
            for text in ('Commit Detail', 'Author', 'Date', 'Parent', head[:7]):
                assert text in screen, screen
            tui.keys('6 j j j')
            assert tui.state()['workspace_focus'] == 'CommitDetail'
            assert tui.state()['detail_scroll'] > 0
            assert 'Body line' in tui.screen()
            tui.keys('G')
            assert 'Body line 29' in tui.screen(), (tui.state(), tui.screen())
            tui.keys('4 j')
            assert 'first subject' in tui.screen()
            assert tui.state()['detail_scroll'] == 0
            tui.keys('k 6 <enter>')
            tui.loaded()
            assert '0 Diff' in tui.screen() and 'Commit Detail' not in tui.screen()
            assert tui.state()['workspace_commit'] == head
            tui.keys('<esc>')
            assert tui.state()['workspace_focus'] == 'CommitDetail'
            assert 'Commit Detail' in tui.screen()
            tui.keys('g')
            screen = tui.screen()
            x, y, width, height = tui.state()['layout']['detail']
            tui.request(cmd='mouse', kind='scroll_down', x=x+2, y=y+2)
            assert tui.state()['detail_scroll'] > 0
            tui.keys('4 ?')
            screen = tui.screen()
            x, y, width, height = tui.state()['layout']['help']
            assert width <= 88 and height <= 24
            selected = tui.state()['selected_commit']
            tui.request(cmd='mouse', kind='click', x=x+2, y=y+2)
            assert 'Help' in tui.screen()
            # Closing must not also activate the underlying graph row.
            tui.request(cmd='mouse', kind='click', x=119, y=3)
            assert 'Help' not in tui.screen()
            assert tui.state()['selected_commit'] == selected
            tui.keys('?')
            wide_help = tui.screen()
            narrow_help = tui.screen(54, 20)
            assert 'Navigation' in narrow_help
            tui.keys('<esc> 6 g')
            narrow_detail = tui.screen(54, 20)
            assert '4 Graph' in narrow_detail and 'Commit Detail' in narrow_detail
            if os.environ.get('LAZYKEIFU_TEST_ARTIFACTS'):
                dest = Path(os.environ['LAZYKEIFU_TEST_ARTIFACTS'])
                dest.mkdir(parents=True, exist_ok=True)
                (dest / 'detail-wide.txt').write_text(tui.screen())
                (dest / 'detail-narrow.txt').write_text(narrow_detail)
                (dest / 'help-wide.txt').write_text(wide_help)
                (dest / 'help-narrow.txt').write_text(narrow_help)
        finally:
            tui.close()
    print('PASS: commit detail metadata, selection, keyboard/wheel scroll, diff return, compact help, inside/outside clicks without click-through')


if __name__ == '__main__':
    run()

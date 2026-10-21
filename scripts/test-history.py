#!/usr/bin/env python3
"""Exercise explicit history loading past 500/1000, including refresh and narrow UI."""
import runpy
import subprocess
import tempfile
from pathlib import Path
w = runpy.run_path(str(Path(__file__).with_name('test-workspace.py')))
Tui, git, init = (w[name] for name in ('Tui', 'git', 'init'))


def run():
    with tempfile.TemporaryDirectory(prefix='lazykeifu-history-') as temp:
        base = Path(temp)
        root = base / 'repo'
        root.mkdir()
        init(root)
        stream = ''.join(f'commit refs/heads/main\ncommitter Test <test@example.invalid> {1700000000+i} +0000\ndata {len(str(i))}\n{i}\n\n' for i in range(1051))
        subprocess.run(['git', 'fast-import', '--quiet'], cwd=root, input=stream, text=True, check=True)
        config = base / 'config'
        (config / 'lazykeifu').mkdir(parents=True)
        (config / 'lazykeifu/config.toml').write_text('[refresh]\nauto_fetch=false\nauto_refresh=false\n')
        tui = Tui(root, config)
        try:
            tui.loaded()
            tui.keys('4 <end>')
            assert tui.state()['history_count'] == 500
            assert tui.state()['history_has_more']
            assert 'Ctrl-l log' in tui.screen()
            selected = tui.state()['selected_commit']
            tui.keys('<c-l> <enter>')
            assert tui.state()['history_count'] == 1000
            assert tui.state()['selected_commit'] == selected
            assert 'Loaded 1000 commits' in tui.screen()
            tui.keys('1 R 4')
            assert tui.state()['history_count'] == 1000
            assert tui.state()['selected_commit'] == selected
            tui.keys('<c-l> <enter>')
            assert tui.state()['history_count'] == 1051
            assert not tui.state()['history_has_more']
            assert tui.state()['selected_commit'] == selected
            assert 'Loaded 1051 commits' in tui.screen(width=60, height=24)
            tui.keys('<end>')
            root_oid = git(root, 'rev-list', '--max-parents=0', 'HEAD').stdout.strip()
            assert tui.state()['selected_commit'] == root_oid[:7]
            tui.keys('<c-l> <enter>')
            assert 'All reachable history is loaded' in tui.screen()
            assert tui.state()['history_count'] == 1051
            # New refs/commits cannot introduce duplicated pages or reset the range.
            git(root, 'commit', '--allow-empty', '-m', 'new head')
            tui.keys('1 R 4')
            assert tui.state()['history_count'] == 1052
            assert tui.state()['selected_commit'] == root_oid[:7]
            assert not tui.state()['history_has_more']
        finally:
            tui.close()
    print('PASS: history 500 → 1000 → 1051, selection, refresh, root access, end-of-history, narrow UI')


if __name__ == '__main__':
    run()

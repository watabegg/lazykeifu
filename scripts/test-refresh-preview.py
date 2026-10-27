#!/usr/bin/env python3
"""Keep the real Diff pane stable during polling, while detecting actual Git changes."""
import runpy
import subprocess
import tempfile
import time
from pathlib import Path

w = runpy.run_path(str(Path(__file__).with_name('test-workspace.py')))
Tui, git, init = (w[name] for name in ('Tui', 'git', 'init'))


def pane(tui):
    screen = tui.screen().splitlines()
    x, y, width, height = tui.state()['layout']['diff']
    return '\n'.join(row[x:x + width] for row in screen[y:y + height])


def stable(tui):
    tui.loaded()
    expected = pane(tui)
    hunk = tui.state()['workspace_hunk']
    end = time.monotonic() + 2.4
    while time.monotonic() < end:
        current = pane(tui)
        assert current == expected, f'Diff changed while idle:\n{expected}\n---\n{current}'
        assert not tui.state()['workspace_preview_loading']
        assert tui.state()['workspace_hunk'] == hunk


def updated(tui, text):
    end = time.monotonic() + 4
    while time.monotonic() < end:
        current = pane(tui)
        assert 'Loading diff' not in current, current
        if text in current and not tui.state()['workspace_preview_loading']:
            return
    raise AssertionError(f'External change was not reflected: {text}\n{pane(tui)}')


def run():
    with tempfile.TemporaryDirectory(prefix='lazykeifu-refresh-preview-') as temp:
        base = Path(temp)
        root = base / 'repo'
        root.mkdir()
        init(root)
        original = [f'line {i:03}\n' for i in range(90)]
        file = root / 'file.txt'
        file.write_text(''.join(original))
        git(root, 'add', '.')
        git(root, 'commit', '-m', 'base')
        working = original.copy()
        working[3] = 'work first\n'
        working[60] = 'work second\n'
        file.write_text(''.join(working))
        config = base / 'config'
        (config / 'lazykeifu').mkdir(parents=True)
        (config / 'lazykeifu/config.toml').write_text('[refresh]\nauto_fetch=false\nrefresh_interval=1\n')
        tui = Tui(root, config, slow_diff=True)
        try:
            tui.keys('2 <enter> 0')
            tui.loaded()
            tui.keys('l')
            assert tui.state()['workspace_hunk'] == 1
            stable(tui)
            working[60] = 'work updated\n'
            file.write_text(''.join(working))
            updated(tui, '+work updated')
            assert tui.state()['workspace_hunk'] == 1
            stable(tui)

            def index_version(first):
                staged = original.copy()
                staged[3] = first + '\n'
                staged[60] = 'index second\n'
                blob = subprocess.run(['git', 'hash-object', '-w', '--stdin'], cwd=root,
                                      input=''.join(staged), text=True, capture_output=True, check=True).stdout.strip()
                git(root, 'update-index', '--cacheinfo', '100644', blob, 'file.txt')

            index_version('index first')
            tui.keys('2 r <enter> <tab>')
            tui.loaded()
            assert tui.state()['workspace_staged']
            assert '+index first' in pane(tui), pane(tui)
            stable(tui)
            mtime = file.stat().st_mtime_ns
            index_version('index changed')
            assert file.stat().st_mtime_ns == mtime
            updated(tui, '+index changed')
            stable(tui)

            # Immutable historical content also stays visible across local polling.
            tui.keys('<esc> 4 <end> <enter> 0')
            tui.loaded()
            tui.keys('j j j')
            stable(tui)
            print('PASS: idle worktree/history diff and hunk/scroll stability, external file edits, index-only changes, no loading flash during refresh')
        finally:
            tui.close()


if __name__ == '__main__':
    run()

#!/usr/bin/env python3
"""Merge/rebase recovery with a real PTY editor handoff and terminal keystrokes."""
import runpy
import shlex
import tempfile
from pathlib import Path

workspace = runpy.run_path(str(Path(__file__).with_name('test-workspace.py')))
Tui, git, init = (workspace[name] for name in ('Tui', 'git', 'init'))


def conflict(root, rebase):
    root.mkdir()
    init(root)
    file = root / '[x] file.txt'
    file.write_text('base\n')
    git(root, 'add', '.')
    git(root, 'commit', '-m', 'base')
    git(root, 'switch', '-c', 'topic')
    file.write_text('topic\n')
    git(root, 'commit', '-am', 'topic')
    git(root, 'switch', 'main')
    file.write_text('main\n')
    git(root, 'commit', '-am', 'main')
    if rebase:
        git(root, 'switch', 'topic')
    original = git(root, 'rev-parse', 'HEAD').stdout
    assert git(root, *(['rebase', 'main'] if rebase else ['merge', 'topic']), check=False).returncode != 0
    return file, original


def run():
    with tempfile.TemporaryDirectory(prefix='lazykeifu-recovery-') as temp:
        base = Path(temp)
        config = base / 'config'
        (config / 'lazykeifu').mkdir(parents=True)
        (config / 'lazykeifu/config.toml').write_text('[refresh]\nauto_fetch=false\nauto_refresh=false\n')
        editor = base / 'fake editor.py'
        ready = base / 'editor-ready'
        used = base / 'commit-editor-used'
        fail = base / 'editor-fail'
        interrupt = base / 'editor-interrupt'
        editor.write_text('''import sys, termios, signal
from pathlib import Path
signal.alarm(6)
base = Path(__file__).parent
flags = termios.tcgetattr(sys.stdin.fileno())[3]
assert flags & termios.ICANON and flags & termios.ECHO, 'TUI did not restore terminal'
if (base / 'editor-fail').exists():
    sys.exit(3)
if (base / 'editor-interrupt').exists():
    (base / 'editor-ready').write_text('ready')
    signal.pause()
file = Path(sys.argv[-1])
if file.name == 'COMMIT_EDITMSG':
    (base / 'commit-editor-used').write_text('yes')
else:
    (base / 'editor-ready').write_text('ready')
    text = sys.stdin.readline()
    assert text == 'resolved from terminal\\n', repr(text)
    file.write_text(text)
''')
        env = {'GIT_EDITOR': f'python3 {shlex.quote(str(editor))}'}
        for rebase in (False, True):
            root = base / ('rebase' if rebase else 'merge')
            file, original = conflict(root, rebase)
            name = 'rebase' if rebase else 'merge'
            tui = Tui(root, config, env_overrides=env, stdin_pipe=True)
            try:
                tui.loaded()
                assert f'{name.upper()}: m continue' in tui.screen()
                tui.keys('m <enter>')
                assert f'Continue {name}?' in tui.screen()
                tui.keys('<enter>')
                tui.wait(lambda: tui.state()['mode'] == 'error')
                assert 'Unresolved conflicts remain' in tui.screen()
                tui.keys('<esc> 2 a')
                assert 'Resolve conflicts individually' in tui.screen()
                tui.keys('<esc> 2 e')
                tui.wait(lambda: ready.exists())
                tui.proc.stdin.write(b'resolved from terminal\n')
                tui.proc.stdin.flush()
                tui.wait(lambda: file.read_text() == 'resolved from terminal\n')
                tui.keys('<enter>')
                tui.loaded()
                assert '+resolved from terminal' in tui.screen()
                # Actual terminal keys prove the old reader released stdin and the new reader works.
                tui.proc.stdin.write(b'4')
                tui.proc.stdin.flush()
                tui.wait(lambda: tui.state()['workspace_focus'] == 'Graph')
                tui.proc.stdin.write(b'2')
                tui.proc.stdin.flush()
                tui.wait(lambda: tui.state()['workspace_focus'] == 'Files')
                tui.loaded()
                tui.keys('<enter> <space>')
                assert 'Mark [x] file.txt resolved' in tui.screen()
                tui.keys('<esc>')
                assert git(root, 'ls-files', '-u').stdout
                tui.keys('<space> <enter>')
                tui.wait(lambda: not tui.state()['workspace_busy'])
                assert not git(root, 'ls-files', '-u').stdout
                tui.keys('m <enter> <enter>')
                tui.wait(lambda: tui.state()['git_operation'] is None)
                assert tui.state()['mode'] == 'normal'
                assert used.exists()
                assert file.read_text() == 'resolved from terminal\n'
                assert git(root, 'rev-parse', 'HEAD').stdout != original
                assert not git(root, 'status', '--porcelain').stdout
                # A failing editor must still restore the app and its input reader.
                fail.write_text('fail')
                file.write_text('working edit\n')
                tui.keys('2 R')
                tui.loaded()
                tui.keys('e')
                tui.wait(lambda: tui.state()['mode'] == 'error')
                assert 'exited with' in tui.screen()
                tui.keys('<esc>')
                tui.proc.stdin.write(b'4')
                tui.proc.stdin.flush()
                tui.wait(lambda: tui.state()['workspace_focus'] == 'Graph')
                fail.unlink()
                ready.unlink()
                interrupt.touch()
                tui.keys('2 e')
                tui.wait(lambda: ready.exists())
                tui.proc.stdin.write(b'\x03')
                tui.proc.stdin.flush()
                tui.wait(lambda: tui.state()['mode'] == 'error')
                screen = tui.screen()
                assert '130' in screen or 'SIGINT' in screen or 'signal: 2' in screen, screen
                assert tui.proc.poll() is None
                tui.keys('<esc>')
                tui.proc.stdin.write(b'4')
                tui.proc.stdin.flush()
                tui.wait(lambda: tui.state()['workspace_focus'] == 'Graph')
                interrupt.unlink()
                ready.unlink()
                used.unlink()
            finally:
                tui.close()

        for rebase in (False, True):
            root = base / ('abort-rebase' if rebase else 'abort-merge')
            file, original = conflict(root, rebase)
            name = 'rebase' if rebase else 'merge'
            tui = Tui(root, config)
            try:
                tui.keys('m j <enter>')
                assert f'Abort {name}?' in tui.screen()
                tui.keys('<esc>')
                assert tui.state()['git_operation'] == name
                tui.keys('m j <enter> <enter>')
                tui.wait(lambda: not tui.state()['workspace_busy'])
                assert tui.state()['git_operation'] is None
                assert git(root, 'rev-parse', 'HEAD').stdout == original
                assert file.read_text() == ('topic\n' if rebase else 'main\n')
            finally:
                tui.close()
        print('PASS: PTY editor stdin/cooked mode/return, failed/cancelled editor recovery, conflict stage confirmation, merge/rebase continue and abort, cancel')


if __name__ == '__main__':
    run()

#!/usr/bin/env python3
"""Selected commit actions, confirmation, and native conflict recovery in the TUI."""
import runpy
import tempfile
from pathlib import Path
w = runpy.run_path(str(Path(__file__).with_name('test-workspace.py')))
Tui, git, init = (w[name] for name in ('Tui', 'git', 'init'))


def run():
    with tempfile.TemporaryDirectory(prefix='pedigit-commit-ops-') as temp:
        base = Path(temp)
        config = base / 'config'
        (config / 'pedigit').mkdir(parents=True)
        (config / 'pedigit/config.toml').write_text('[refresh]\nauto_fetch=false\nauto_refresh=false\n')
        for revert in (False, True):
            for outcome in ('clean', 'continue', 'abort'):
                root = base / f'repo-{revert}-{outcome}'
                root.mkdir()
                init(root)
                file = root / 'file'
                file.write_text('base\n')
                git(root, 'add', '.')
                git(root, 'commit', '-m', 'base')
                git(root, 'switch', '-c', 'topic')
                file.write_text('topic\n')
                git(root, 'commit', '-am', 'topic change')
                target = git(root, 'rev-parse', 'HEAD').stdout.strip()
                git(root, 'switch', 'main')
                if revert:
                    git(root, 'merge', '--ff-only', 'topic')
                if outcome != 'clean':
                    file.write_text('different\n')
                    git(root, 'commit', '-am', 'different')
                original = git(root, 'rev-parse', 'HEAD').stdout
                key, name = ('t', 'revert') if revert else ('C V', 'cherry-pick')
                tui = Tui(root, config, env_overrides={'GIT_EDITOR': 'true'})
                try:
                    tui.loaded()
                    tui.keys('2 ' + key)
                    assert tui.state()['mode'] == 'normal'
                    tui.keys('/ t o p i c <enter> 4 ' + key)
                    assert tui.state()['mode'] == 'confirm', tui.screen()
                    screen = tui.screen(width=60, height=24)
                    assert name in screen and target[:7] in screen and "'main'" in screen, screen
                    tui.keys('<esc>')
                    assert git(root, 'rev-parse', 'HEAD').stdout == original
                    tui.keys(key + ' <enter>')
                    if outcome != 'clean':
                        tui.wait(lambda: tui.state()['mode'] == 'error')
                        tui.keys('<esc> 2')
                        tui.loaded()
                        assert f'{name.upper()}: m continue' in tui.screen(), tui.screen()
                        if outcome == 'abort':
                            tui.keys('m j <enter>')
                            assert f'Abort {name}?' in tui.screen()
                            tui.keys('<enter>')
                            tui.wait(lambda: not tui.state()['workspace_busy'])
                            assert git(root, 'rev-parse', 'HEAD').stdout == original
                            assert file.read_text() == 'different\n'
                        else:
                            file.write_text('resolved\n')
                            tui.keys('R')
                            tui.loaded()
                            tui.keys('<space> <enter>')
                            tui.wait(lambda: not tui.state()['workspace_busy'])
                            tui.keys('m <enter>')
                            assert f'Continue {name}?' in tui.screen()
                            tui.keys('<enter>')
                    if outcome != 'abort':
                        tui.wait(lambda: git(root, 'rev-parse', 'HEAD').stdout != original)
                    tui.wait(lambda: tui.state()['mode'] == 'normal')
                    assert tui.state()['git_operation'] is None
                    if outcome == 'clean':
                        assert file.read_text() == ('base\n' if revert else 'topic\n')
                    assert git(root, 'rev-parse', 'topic').stdout.strip() == target
                    assert f'{name.upper()}: m continue' not in tui.screen()
                finally:
                    tui.close()
    print('PASS: cherry-pick/revert selection, cancel, narrow confirmation, clean execution, conflict continue/abort')


if __name__ == '__main__':
    run()

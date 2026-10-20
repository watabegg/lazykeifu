#!/usr/bin/env python3
"""Start merge/rebase from the real branch and graph panels, then recover."""
import runpy
import tempfile
from pathlib import Path

workspace = runpy.run_path(str(Path(__file__).with_name('test-workspace.py')))
Tui, git, init = (workspace[name] for name in ('Tui', 'git', 'init'))


def run():
    with tempfile.TemporaryDirectory(prefix='lazykeifu-integrate-') as temp:
        base = Path(temp)
        config = base / 'config'
        (config / 'lazykeifu').mkdir(parents=True)
        (config / 'lazykeifu/config.toml').write_text('[refresh]\nauto_fetch=false\nauto_refresh=false\n')
        for rebase in (False, True):
            for conflict in (False, True):
                root = base / f'repo-{rebase}-{conflict}'
                root.mkdir()
                init(root)
                file = root / 'file'
                file.write_text('base\n')
                git(root, 'add', '.')
                git(root, 'commit', '-m', 'base')
                git(root, 'switch', '-c', 'topic')
                file.write_text('topic\n')
                git(root, 'commit', '-am', 'topic')
                git(root, 'switch', 'main')
                (file if conflict else root / 'other').write_text('main\n')
                git(root, 'add', '.')
                git(root, 'commit', '-m', 'main')
                original = git(root, 'rev-parse', 'HEAD').stdout
                target = git(root, 'rev-parse', 'topic').stdout
                key, name = ('r', 'rebase') if rebase else ('M', 'merge')
                tui = Tui(root, config, env_overrides={'GIT_EDITOR': 'true'})
                try:
                    tui.loaded()
                    tui.keys('3 j ' + key)
                    screen = tui.screen()
                    assert tui.state()['mode'] == 'confirm', screen
                    assert "current branch 'main'" in screen and "'topic'" in screen, screen
                    assert ('onto' if rebase else 'into') in screen, screen
                    if rebase:
                        narrow = tui.screen(width=60, height=24)
                        assert 'rewritten.' in narrow, narrow
                    tui.keys('<esc>')
                    assert git(root, 'rev-parse', 'HEAD').stdout == original
                    # Search selects the branch in the graph; the same action works there.
                    tui.keys('/ t o p i c <enter> 4 m ' + ('j ' if rebase else '') + '<enter>')
                    assert tui.state()['mode'] == 'confirm', tui.screen()
                    assert "'topic'" in tui.screen()
                    tui.keys('<enter>')
                    if conflict:
                        tui.wait(lambda: tui.state()['mode'] == 'error')
                        tui.keys('<esc> 2')
                        tui.loaded()
                        assert f'{name.upper()}: m continue' in tui.screen(), tui.screen()
                        file.write_text('resolved\n')
                        tui.keys('R')
                        tui.loaded()
                        tui.keys('<space>')
                        assert 'Mark file resolved' in tui.screen(), tui.screen()
                        tui.keys('<enter>')
                        tui.wait(lambda: not tui.state()['workspace_busy'])
                        tui.keys('m <enter>')
                        assert f'Continue {name}?' in tui.screen()
                        tui.keys('<enter>')
                    tui.wait(lambda: git(root, 'rev-parse', 'HEAD').stdout != original)
                    tui.wait(lambda: tui.state()['mode'] == 'normal')
                    assert f'{name.upper()}: m continue' not in tui.screen()
                    assert git(root, 'branch', '--show-current').stdout.strip() == 'main'
                    assert git(root, 'rev-parse', 'topic').stdout == target
                    # A stale confirmation cannot operate after another process moves the branch.
                    tui.keys('3 <home> j ' + key)
                    assert tui.state()['mode'] == 'confirm', tui.screen()
                    git(root, 'update-ref', 'refs/heads/topic', 'HEAD')
                    tui.keys('<enter>')
                    tui.wait(lambda: tui.state()['mode'] == 'error')
                    assert 'changed; review and confirm again' in tui.screen(), tui.screen()
                finally:
                    tui.close()
        print('PASS: merge/rebase branch and graph actions, direction, cancel, conflicts, continue, stale targets')


if __name__ == '__main__':
    run()

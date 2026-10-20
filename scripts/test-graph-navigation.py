#!/usr/bin/env python3
"""Graph keyboard navigation skips connector rows without selecting working changes."""
import runpy
import tempfile
from pathlib import Path

w = runpy.run_path(str(Path(__file__).with_name('test-workspace.py')))
Tui, git, init = (w[name] for name in ('Tui', 'git', 'init'))


def run():
    with tempfile.TemporaryDirectory(prefix='pedigit-navigation-') as temp:
        base = Path(temp)
        root = base / 'repo'
        root.mkdir()
        init(root)

        def commit(name):
            (root / name).write_text(name + '\n')
            git(root, 'add', '.')
            git(root, 'commit', '-m', name)

        commit('base')
        git(root, 'switch', '-c', 'topic')
        commit('topic-file')
        git(root, 'switch', 'main')
        commit('main-file')
        git(root, 'merge', '--no-ff', 'topic', '-m', 'merge')
        (root / 'working-only').write_text('working\n')
        config = base / 'config'
        (config / 'pedigit').mkdir(parents=True)
        (config / 'pedigit/config.toml').write_text('[refresh]\nauto_fetch=false\nauto_refresh=false\n')
        tui = Tui(root, config)
        try:
            for down, up in [('j', 'k'), ('<down>', '<up>')]:
                tui.keys('4 <home>')
                tui.loaded()
                assert 'working-only' in tui.screen()
                indices, commits = [0], []
                for _ in range(4):
                    tui.keys(down)
                    tui.loaded()
                    state = tui.state()
                    assert state['workspace_commit'], tui.screen()
                    assert 'working-only' not in tui.screen(), tui.screen()
                    indices.append(state['selected_index'])
                    commits.append(state['workspace_commit'])
                assert len(set(commits)) == 4, commits
                assert any(b - a > 1 for a, b in zip(indices, indices[1:])), indices
                tui.keys(down)
                assert tui.state()['selected_index'] == indices[-1]
                for i in range(3, -1, -1):
                    tui.keys(up)
                    tui.loaded()
                    assert tui.state()['selected_index'] == indices[i], tui.screen()
                    if i:
                        assert tui.state()['workspace_commit'] == commits[i - 1]
                        assert 'working-only' not in tui.screen()
                assert tui.state()['workspace_commit'] is None
                assert 'working-only' in tui.screen()
                tui.keys(up)
                assert tui.state()['selected_index'] == 0
        finally:
            tui.close()
    print('PASS: j/k and arrows skip graph connectors in both directions, retain commit files, preserve working-tree node and boundaries')


if __name__ == '__main__':
    run()

#!/usr/bin/env python3
"""Drive every embedded language through actual TUI drawing and hit testing."""
import json
import os
import unicodedata
import runpy
import subprocess
import tempfile
from pathlib import Path

w = runpy.run_path(str(Path(__file__).with_name('test-workspace.py')))
Tui, git, init, BINARY = (w[n] for n in ('Tui', 'git', 'init', 'BINARY'))
ROOT = Path(__file__).resolve().parents[1]
LANGUAGES = ('en', 'ja', 'ko', 'nl', 'pl', 'pt', 'ru', 'zh-CN', 'zh-TW')


def render(tui, *size):
    # Debug dumps include the blank continuation cell of each wide glyph.
    lines = []
    for line in tui.screen(*size).splitlines():
        result = []
        skip = False
        for char in line:
            if skip:
                skip = False
                if char == ' ':
                    continue
            result.append(char)
            skip = unicodedata.east_asian_width(char) in ('W', 'F')
        lines.append(''.join(result))
    return '\n'.join(lines)


def run():
    with tempfile.TemporaryDirectory(prefix='lazykeifu-i18n-') as temp:
        base = Path(temp)
        root = base / 'repo'
        root.mkdir()
        init(root)
        # These look like UI text or placeholders but must remain repository data.
        filename = 'Files-{name}-日本語.txt'
        (root / filename).write_text('base\n')
        git(root, 'add', '.')
        git(root, 'commit', '-m', 'Confirm {name} 日本語')
        (root / filename).write_text('modified\n')
        git(root, 'branch', 'Files')
        config = base / 'config'
        (config / 'lazykeifu').mkdir(parents=True)
        config_file = config / 'lazykeifu/config.toml'

        def configure(language):
            config_file.write_text(f'[ui]\nlanguage="{language}"\n[refresh]\nauto_fetch=false\nauto_refresh=false\n')

        for language in LANGUAGES:
            configure(language)
            catalog = json.loads((ROOT / f'locales/{language}.json').read_text())
            tui = Tui(root, config)
            try:
                assert tui.state()['language'] == language
                tui.loaded()
                screen = render(tui)
                if os.environ.get('LAZYKEIFU_TEST_ARTIFACTS'):
                    evidence = Path(os.environ['LAZYKEIFU_TEST_ARTIFACTS'])
                    evidence.mkdir(parents=True, exist_ok=True)
                    (evidence / f'{language}-workspace.txt').write_text(screen)
                assert filename in screen, screen
                assert 'Confirm {name} 日本語' in screen, screen
                # All tabs share display-width-based hitboxes even with CJK glyphs.
                tui.keys('2 ]')
                assert tui.state()['files_tab'] == 'Worktrees'
                tui.keys('[')
                assert tui.state()['files_tab'] == 'Files'
                for label, expected in [('Worktrees', 'Worktrees'), ('Files', 'Files')]:
                    header = render(tui).splitlines()[1]
                    translated = catalog.get(label, label)
                    before, found, _ = header.partition(translated)
                    assert found, header
                    def width(text):
                        return sum(0 if unicodedata.combining(c) else 2 if unicodedata.east_asian_width(c) in ('W', 'F') else 1 for c in text)
                    x = width(before) + width(translated) - 1
                    tui.request(cmd='mouse', kind='click', x=x, y=1)
                    assert tui.state()['files_tab'] == expected, render(tui)
                # Dialogs and help render in wide and narrow terminals without panics.
                tui.keys('c')
                assert tui.state()['mode'] == 'confirm', render(tui)
                for width, height in ((120, 32), (80, 24), (54, 20)):
                    screen = render(tui, width, height)
                    if os.environ.get('LAZYKEIFU_TEST_ARTIFACTS'):
                        (evidence / f'{language}-confirm-{width}.txt').write_text(screen)
                    assert 'Enter' in screen, screen
                    if language != 'en':
                        assert 'Nothing staged' not in screen, screen
                tui.keys('<esc> ?')
                for width, height in ((120, 32), (54, 20), (24, 10)):
                    screen = render(tui, width, height)
                    assert screen.strip(), screen
                tui.keys('<esc>')
                assert tui.state()['mode'] == 'normal'
                # Pull starts immediately; this repository has no upstream.
                tui.keys('4 <end> p')
                assert tui.state()['mode'] != 'confirm', render(tui)
                tui.wait(lambda: not tui.state()['workspace_busy'])
                assert tui.state()['mode'] == 'error', render(tui)
                tui.keys('<esc> P')
                assert tui.state()['mode'] == 'confirm', render(tui)
                expected = catalog["Push current branch '{branch}'?"].replace('{branch}', 'main')
                assert expected in render(tui), render(tui)
                tui.keys('<esc> <space>')
                assert tui.state()['mode'] == 'git_menu', render(tui)
                screen = render(tui)
                for message in ('Cancel', 'Stash changes and switch', 'Discard all changes and switch'):
                    assert catalog[message] in screen, screen
                assert '› ' + catalog['Cancel'] in screen, screen
                tui.keys('<enter>')
                assert tui.state()['mode'] == 'normal', render(tui)
                assert (root / filename).read_text() == 'modified\n'
                assert git(root, 'symbolic-ref', '--short', 'HEAD').stdout.strip() == 'main'
                # Stage through keyboard, then inspect localized commit composer.
                tui.keys('2 <space> c')
                tui.loaded()
                assert tui.state()['mode'] == 'input', render(tui)
                assert filename in render(tui), render(tui)
                tui.keys('<esc> 2 a')
                assert not git(root, 'diff', '--cached', '--name-only').stdout
                # Confirm there are actually translated app labels, not English catalogs.
                if language != 'en':
                    assert any(value != key and value.strip() in render(tui) for key, value in catalog.items() if len(value.strip()) >= 3)
            finally:
                tui.close()

        configure('auto')
        for env, expected in [({'LC_ALL': 'ja_JP.UTF-8'}, 'ja'), ({'LC_ALL': 'zh_Hant_TW.UTF-8'}, 'zh-TW'), ({'LC_ALL': 'zz_ZZ.UTF-8'}, 'en')]:
            tui = Tui(root, config, env_overrides=env)
            try:
                assert tui.state()['language'] == expected
                assert render(tui).strip()
            finally:
                tui.close()
        configure('ja')
        tui = Tui(root, config, cli_args=('--language', 'en'))
        try:
            assert tui.state()['language'] == 'en'
            assert 'Files' in render(tui)
        finally:
            tui.close()
        result = subprocess.run([str(BINARY), '--language', 'unknown'], cwd=root, capture_output=True, text=True)
        assert result.returncode != 0 and 'Unsupported UI language' in result.stderr
    print('PASS: nine languages, original repository content, dialogs/composer/help, locale detection/fallback, CLI precedence and invalid language')


if __name__ == '__main__':
    run()

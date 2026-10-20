#!/usr/bin/env python3
"""Extract literal translation keys and check all embedded catalogs (stdlib only)."""
import argparse
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LANGUAGES = ('ja', 'ko', 'nl', 'pl', 'pt', 'ru', 'zh-CN', 'zh-TW')
LITERAL = re.compile(r'\btrf?!\s*\(\s*("(?:[^"\\]|\\.)*")', re.S)
PLACEHOLDER = re.compile(r'\{([a-zA-Z_][a-zA-Z_0-9]*)\}')


def extract():
    result = {}
    for path in (ROOT / 'src').rglob('*.rs'):
        if path.name == 'i18n.rs':
            continue
        source = path.read_text().split('#[cfg(test)]')[0]
        for match in LITERAL.finditer(source):
            key = json.loads(match[1])
            result[key] = key
    return dict(sorted(result.items()))


def check():
    english = json.loads((ROOT / 'locales/en.json').read_text())
    assert english == extract(), 'Run python3 scripts/i18n-catalog.py --extract after changing UI text'
    for language in LANGUAGES:
        path = ROOT / f'locales/{language}.json'
        def unique(pairs):
            result = {}
            for key, value in pairs:
                assert key not in result, f'{path}: duplicate key {key!r}'
                result[key] = value
            return result
        catalog = json.loads(path.read_text(), object_pairs_hook=unique)
        assert catalog.keys() == english.keys(), f'{language}: missing {english.keys() - catalog.keys()}; extra {catalog.keys() - english.keys()}'
        for key, text in catalog.items():
            assert isinstance(text, str) and text.strip(), f'{language}: empty {key!r}'
            assert sorted(PLACEHOLDER.findall(key)) == sorted(PLACEHOLDER.findall(text)), f'{language}: placeholders differ in {key!r}: {text!r}'
            assert len(key) - len(key.lstrip()) == len(text) - len(text.lstrip()), f'{language}: leading whitespace in {key!r}'
            assert len(key) - len(key.rstrip()) == len(text) - len(text.rstrip()), f'{language}: trailing whitespace in {key!r}'
    print(f'PASS: {len(english)} UI messages across {len(LANGUAGES) + 1} languages; keys, placeholders, whitespace')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--extract', action='store_true')
    args = parser.parse_args()
    if args.extract:
        catalog = extract()
        (ROOT / 'locales/en.json').write_text(json.dumps(catalog, ensure_ascii=False, indent=2) + '\n')
        print(f'Extracted {len(catalog)} messages')
    else:
        check()

#!/usr/bin/env python3
"""Package a native, smoke-tested binary with notices and a checksum."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tarfile
import zipfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--tag', default='')
    parser.add_argument('--target')
    parser.add_argument('--check-only', action='store_true')
    args = parser.parse_args()
    metadata = json.loads(subprocess.check_output([
        'cargo', 'metadata', '--no-deps', '--locked', '--format-version', '1'
    ]))
    package = next(p for p in metadata['packages'] if p['name'] == 'lazykeifu')
    version = package['version']
    if args.tag and args.tag != f'lazykeifu-v{version}':
        parser.error(f'tag must be lazykeifu-v{version}; upstream v* tags are not releases')
    if args.check_only:
        return
    if not args.target:
        parser.error('--target is required when packaging')
    windows = 'windows' in args.target
    binary_name = 'lzkf.exe' if windows else 'lzkf'
    binary = Path('target') / args.target / 'release' / binary_name
    result = subprocess.check_output([str(binary.resolve()), '--version'], text=True).strip()
    if result != f'lzkf {version}':
        raise RuntimeError(f'Unexpected binary version: {result}')
    files = [(binary, binary_name), (Path('LICENSE'), 'LICENSE'),
             (Path('locales/LICENSE.lazygit'), 'LICENSE.lazygit'),
             (Path('THIRD_PARTY.json'), 'THIRD_PARTY.json'), (Path('README.md'), 'README.md')]
    for source, _ in files:
        if not source.is_file():
            raise FileNotFoundError(source)
    output = Path('dist')
    output.mkdir(exist_ok=True)
    archive = output / f'lzkf-{version}-{args.target}{".zip" if windows else ".tar.gz"}'
    if windows:
        with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED) as bundle:
            for source, name in files:
                bundle.write(source, name)
    else:
        with tarfile.open(archive, 'w:gz') as bundle:
            for source, name in files:
                bundle.add(source, arcname=name)
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    archive.with_name(archive.name + '.sha256').write_text(f'{digest}  {archive.name}\n')
    print(archive)


if __name__ == '__main__':
    main()

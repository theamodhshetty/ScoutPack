#!/usr/bin/env python3
"""Package release binaries and smoke-test the extracted artifact (stdlib only)."""
import argparse
import hashlib
from pathlib import Path
import subprocess
import tarfile
import tempfile
import tomllib
import zipfile

TARGETS = (
    'aarch64-apple-darwin', 'x86_64-apple-darwin',
    'aarch64-unknown-linux-gnu', 'x86_64-unknown-linux-gnu',
    'x86_64-pc-windows-msvc',
)
ROOT = Path(__file__).resolve().parent.parent


def package(target, output):
    output.mkdir(parents=True, exist_ok=True)
    stem = f'scoutpack-{target}'
    binary = 'scoutpack.exe' if 'windows' in target else 'scoutpack'
    files = {binary: ROOT / 'target' / target / 'release' / binary,
             'README.md': ROOT / 'README.md', 'LICENSE': ROOT / 'LICENSE'}
    archive = output / (stem + ('.zip' if 'windows' in target else '.tar.gz'))
    if 'windows' in target:
        with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED) as bundle:
            for name, source in files.items():
                bundle.write(source, f'{stem}/{name}')
    else:
        with tarfile.open(archive, 'w:gz') as bundle:
            for name, source in files.items():
                bundle.add(source, arcname=f'{stem}/{name}')
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    archive.with_name(archive.name + '.sha256').write_text(f'{digest}  {archive.name}\n')
    return archive


def verify(target, archive):
    archive = archive.resolve()
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    expected = archive.with_name(archive.name + '.sha256').read_text().split()
    if expected != [digest, archive.name]:
        raise ValueError('archive checksum does not match sidecar')
    stem = f'scoutpack-{target}'
    binary = 'scoutpack.exe' if 'windows' in target else 'scoutpack'
    expected_names = {f'{stem}/{name}' for name in (binary, 'README.md', 'LICENSE')}
    with tempfile.TemporaryDirectory(prefix='scoutpack-release-') as temporary:
        root = Path(temporary)
        # Read only the three expected regular members; never extract archive paths.
        if archive.suffix == '.zip':
            with zipfile.ZipFile(archive) as bundle:
                if set(bundle.namelist()) != expected_names or len(bundle.infolist()) != 3:
                    raise ValueError('unexpected archive contents')
                for name in expected_names:
                    (root / Path(name).name).write_bytes(bundle.read(name))
        else:
            with tarfile.open(archive) as bundle:
                members = bundle.getmembers()
                if {m.name for m in members} != expected_names or len(members) != 3:
                    raise ValueError('unexpected archive contents')
                for member in members:
                    if not member.isfile():
                        raise ValueError('non-regular archive member')
                    (root / Path(member.name).name).write_bytes(bundle.extractfile(member).read())
        executable = root / binary
        executable.chmod(0o755)
        version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']['version']
        def run(*args, cwd=root):
            result = subprocess.run([str(executable), *args], cwd=cwd, check=True,
                                    capture_output=True, text=True, timeout=60)
            print(result.stdout, end='')
            return result.stdout
        if run('--version').strip() != f'scoutpack {version}':
            raise ValueError('packaged executable version differs from Cargo.toml')
        fixture = root / 'fixture'
        fixture.mkdir()
        marker = 'RELEASE_SMOKE_MARKER'
        (fixture / 'main.py').write_text(f'def release_probe():\n    return "{marker}"\n')
        run('pack', str(fixture))
        packet = run('context', 'release_probe', '--budget', '1500', cwd=fixture)
        if marker not in packet:
            raise ValueError('installed executable did not retrieve the fixture')
        run('doctor', str(fixture))
        print(f'Verified {target}: {digest}')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['package', 'verify'])
    parser.add_argument('--target', required=True, choices=TARGETS)
    parser.add_argument('--output', type=Path, default=Path('release-assets'))
    parser.add_argument('--archive', type=Path)
    args = parser.parse_args()
    if args.action == 'package':
        print(package(args.target, args.output))
    elif args.archive:
        verify(args.target, args.archive)
    else:
        parser.error('verify requires --archive')

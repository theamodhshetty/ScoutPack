"""Offline installer regressions: synthetic artifacts, no GitHub downloads."""
import hashlib
import io
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


@unittest.skipIf(os.name == 'nt', 'Unix installer')
class InstallTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='scoutpack install ')
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.downloads = self.root / 'downloads'
        self.downloads.mkdir()
        self.bin = self.root / 'mock-bin'
        self.bin.mkdir()
        # Keep the actual hashing, tar, chmod, mkdir and copy tools. Only the
        # transport and platform detection are replaced for an offline fixture.
        for name, script in {
            'uname': '#!/bin/sh\ncase "$1" in -s) echo Linux;; -m) echo x86_64;; esac\n',
            'curl': '#!/bin/sh\ncp "$MOCK_DOWNLOADS/${2##*/}" "$4"\n',
        }.items():
            path = self.bin / name
            path.write_text(script)
            path.chmod(0o755)
        self.install = self.root / 'install dir'
        self.install.mkdir()
        self.destination = self.install / 'scoutpack'
        self.destination.write_text('original installation')
        self.env = dict(os.environ, PATH=str(self.bin) + os.pathsep + os.environ['PATH'],
                        MOCK_DOWNLOADS=str(self.downloads))
        self.archive = self.downloads / 'scoutpack-x86_64-unknown-linux-gnu.tar.gz'
        with tarfile.open(self.archive, 'w:gz') as bundle:
            data = b'#!/bin/sh\necho "scoutpack test-release"\n'
            member = tarfile.TarInfo('scoutpack-x86_64-unknown-linux-gnu/scoutpack')
            member.size = len(data)
            member.mode = 0o755
            bundle.addfile(member, io.BytesIO(data))
        self.checksum = self.downloads / (self.archive.name + '.sha256')
        self.valid = f'{hashlib.sha256(self.archive.read_bytes()).hexdigest()}  {self.archive.name}\n'
        self.checksum.write_text(self.valid)

    def run_installer(self):
        return subprocess.run(['sh', str(ROOT / 'install.sh'), '--version', 'v-test',
                               '--dir', str(self.install)], env=self.env,
                              capture_output=True, text=True, timeout=10)

    def assert_preserved(self):
        self.assertEqual(self.destination.read_text(), 'original installation')

    def test_valid_install_in_directory_with_spaces(self):
        result = self.run_installer()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('scoutpack test-release', result.stdout)
        self.assertTrue(os.access(self.destination, os.X_OK))

    def test_tampered_archive_preserves_existing_install(self):
        self.archive.write_bytes(self.archive.read_bytes() + b'corruption')
        result = self.run_installer()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('checksum mismatch', result.stderr)
        self.assert_preserved()

    def test_missing_checksum_preserves_existing_install(self):
        self.checksum.unlink()
        self.assertNotEqual(self.run_installer().returncode, 0)
        self.assert_preserved()

    def test_wrong_name_duplicate_or_invalid_hash_is_rejected(self):
        for content in [self.valid.replace(self.archive.name, 'other.tar.gz'),
                        self.valid * 2, 'not-a-hash  ' + self.archive.name + '\n']:
            with self.subTest(content=content):
                self.checksum.write_text(content)
                result = self.run_installer()
                self.assertNotEqual(result.returncode, 0)
                self.assertIn('invalid checksum', result.stderr)
                self.assert_preserved()

    @unittest.skipUnless(shutil.which('shasum'), 'requires shasum fallback')
    def test_shasum_fallback(self):
        # A minimal PATH with no sha256sum also covers macOS's hashing route.
        for name in ('sh', 'awk', 'tar', 'gzip', 'mktemp', 'cp', 'chmod', 'mkdir', 'rm', 'mv', 'shasum', 'perl'):
            source = shutil.which(name)
            if source:
                (self.bin / name).symlink_to(source)
        self.env['PATH'] = str(self.bin)
        result = self.run_installer()
        self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == '__main__':
    unittest.main()

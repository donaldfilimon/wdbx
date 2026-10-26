"""Isolated preservation identity and refusal tests; no live stores."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / 'check_preservation.py'


class PreservationTests(unittest.TestCase):
    def setUp(self):
        spec = importlib.util.spec_from_file_location('preservation', SCRIPT)
        self.module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.module)
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.manifest = {'version': 1, 'profiles': {}}
        self.pins = {}
        for profile, info in self.module.PROFILES.items():
            original = json.loads((SCRIPT.parents[2] / info['baseline']).read_text())
            rows = []
            changes = []
            for row in original['files']:
                path = row['path']
                data = ('synthetic:' + path).encode()
                if profile == 'scoring' and path == self.module.ACCESSOR_PATH:
                    data += self.module.ACCESSOR
                rows.append({'path': path, 'sha256': self.digest(data)})
                if path in info['changed']:
                    current = data + b':reviewed'
                    changes.append({'path': path, 'prior_sha256': self.digest(data),
                                    'current_sha256': self.digest(current),
                                    'provenance': ['a' * 40], 'rationale': 'reviewed fixture'})
                    data = current
                if profile == 'language' and path == self.module.ACCESSOR_PATH:
                    data += self.module.ACCESSOR
                target = self.root / path
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(data)
            raw = json.dumps({'files': rows}).encode()
            target = self.root / info['baseline']
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(raw)
            self.pins[profile] = self.digest(raw)
            self.manifest['profiles'][profile] = {'baseline_sha256': self.digest(raw), 'changes': changes}
        # Shared fixture paths have identical synthetic/current content in both profiles.
        self.write_manifest()

    @staticmethod
    def digest(data):
        return hashlib.sha256(data).hexdigest()

    def write_manifest(self):
        raw = json.dumps(self.manifest).encode()
        target = self.root / self.module.TRANSITION_PATH
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(raw)
        self.manifest_pin = self.digest(raw)

    def check(self, profile='language', mode='current'):
        with patch.object(self.module, 'BASELINE_PINS', self.pins), patch.object(
                self.module, 'TRANSITION_SHA256', self.manifest_pin):
            self.module.check(self.root, profile, mode)

    def test_reviewed_current(self):
        for profile in self.module.PROFILES:
            self.check(profile)

    def test_historical_refuses_reviewed_changes(self):
        for profile in self.module.PROFILES:
            with self.assertRaises(ValueError):
                self.check(profile, 'historical')

    def test_each_changed_file_refuses_mutation_or_missing(self):
        for profile, info in self.module.PROFILES.items():
            for path in info['changed']:
                target = self.root / path
                original = target.read_bytes()
                for data in (b'corrupt', None):
                    if data is None:
                        target.unlink()
                    else:
                        target.write_bytes(data)
                    with self.assertRaises(ValueError, msg=path):
                        self.check(profile)
                    target.write_bytes(original)

    def test_untouched_file_refuses(self):
        (self.root / 'specimen-studio/lib/specimen/native.ts').write_bytes(b'changed')
        for profile in self.module.PROFILES:
            with self.assertRaises(ValueError):
                self.check(profile)

    def test_transition_shape_refuses_even_with_matching_pin(self):
        saved = copy.deepcopy(self.manifest)
        for profile in self.module.PROFILES:
            for kind in ('missing', 'duplicate', 'extra', 'prior', 'hash', 'fields'):
                self.manifest = copy.deepcopy(saved)
                rows = self.manifest['profiles'][profile]['changes']
                if kind == 'missing': rows.pop()
                elif kind == 'duplicate': rows.append(copy.deepcopy(rows[0]))
                elif kind == 'extra': rows[0]['path'] = 'extra.rs'
                elif kind == 'prior': rows[0]['prior_sha256'] = '0' * 64
                elif kind == 'hash': rows[0]['current_sha256'] = 'invalid'
                else: rows[0]['surprise'] = True
                self.write_manifest()
                with self.assertRaises(ValueError, msg=kind): self.check(profile)

    def test_malformed_and_duplicate_json_refuse(self):
        target = self.root / self.module.TRANSITION_PATH
        for raw in (b'{', b'{"version":1,"version":1,"profiles":{}}', b'[]'):
            target.write_bytes(raw)
            self.manifest_pin = self.digest(raw)
            with self.assertRaises((ValueError, TypeError)):
                self.check()

    def test_historical_accepts_exact_original_rows_with_accessor(self):
        for profile, info in self.module.PROFILES.items():
            saved = {}
            for path in info['changed']:
                target = self.root / path
                saved[path] = target.read_bytes()
                target.write_bytes(('synthetic:' + path).encode())
            self.check(profile, 'historical')
            for path, data in saved.items():
                (self.root / path).write_bytes(data)

    def test_transition_profile_and_baseline_pin_refuse(self):
        saved = copy.deepcopy(self.manifest)
        for kind in ('extra', 'missing', 'baseline'):
            self.manifest = copy.deepcopy(saved)
            profiles = self.manifest['profiles']
            if kind == 'extra':
                profiles['unexpected'] = copy.deepcopy(profiles['language'])
            elif kind == 'missing':
                del profiles['scoring']
            else:
                profiles['language']['baseline_sha256'] = '0' * 64
            self.write_manifest()
            with self.assertRaises(ValueError):
                self.check()

    def test_manifest_tampering_refuses(self):
        with (self.root / self.module.TRANSITION_PATH).open('ab') as handle:
            handle.write(b' ')
        with self.assertRaises(ValueError): self.check()

    def test_baseline_tampering_refuses(self):
        for profile, info in self.module.PROFILES.items():
            with (self.root / info['baseline']).open('ab') as handle: handle.write(b' ')
            with self.assertRaises(ValueError): self.check(profile)

    def test_accessor_is_exactly_once_and_language_only(self):
        target = self.root / self.module.ACCESSOR_PATH
        original = target.read_bytes()
        for data in (original.replace(self.module.ACCESSOR, b''),
                     original + self.module.ACCESSOR,
                     original.replace(b'O(1)', b'O(2)')):
            target.write_bytes(data)
            with self.assertRaises(ValueError): self.check()
        target.write_bytes(original)
        self.check()


if __name__ == '__main__':
    unittest.main()

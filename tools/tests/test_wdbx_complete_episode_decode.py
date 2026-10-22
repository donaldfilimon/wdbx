"""Final private pipeline and finite JSON-lines protocol tests."""
import hashlib
import io
import json
from pathlib import Path
import sys
import unittest
from unittest.mock import patch
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import wdbx_complete_episode_decode as d

FIXTURES = Path(__file__).resolve().parents[2] / 'crates/abi-wdbx/src/v3/complete_episode/tests'

class CompleteTests(unittest.TestCase):
    def test_pinned_minimal_and_populated(self):
        for name in ('minimal', 'populated'):
            data = bytes.fromhex((FIXTURES / f'provisional-{name}.hex').read_text())
            expected = json.loads((FIXTURES / f'provisional-{name}.json').read_text())
            if name == 'populated':
                expected['context']['regime_posterior']['known'] = [['a', 1000000], ['z', 0]]
            actual = d.decode_complete(data)
            self.assertEqual(actual, {'fixture': expected, 'bytes': data.hex(),
                                      'digest': hashlib.sha256(data).hexdigest()})

if __name__ == '__main__':
    unittest.main()

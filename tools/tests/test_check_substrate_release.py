"""Release C0 mutation tests; no product evidence is asserted."""
import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('release', ROOT / 'tools/check_substrate_release.py')
checker = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(checker)


class ReleaseContract(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        shutil.copytree(ROOT / 'docs/completion/substrate-release', self.root / 'docs/completion/substrate-release')
        self.directory = self.root / 'docs/completion/substrate-release'

    def mutate(self, name, change):
        path = self.directory / name
        value = json.loads(path.read_text())
        change(value)
        path.write_text(json.dumps(value))

    def rejected(self):
        self.assertTrue(checker.validate(self.root, live=False))

    def test_valid(self):
        self.assertEqual([], checker.validate(self.root, live=False))

    def test_missing_requirement(self):
        self.mutate('inventory-v1.json', lambda v: v['obligations'].pop())
        self.rejected()

    def test_coordinated_shrink(self):
        self.mutate('catalog-v1.json', lambda v: v['items'].pop())
        self.mutate('inventory-v1.json', lambda v: v['obligations'].pop())
        self.rejected()

    def test_duplicate(self):
        self.mutate('inventory-v1.json', lambda v: v['obligations'].append(v['obligations'][0]))
        self.rejected()

    def test_source(self):
        with (self.directory / 'sources/plan.md').open('a') as stream:
            stream.write('\nchanged\n')
        self.rejected()

    def test_fabricated_success(self):
        self.mutate('inventory-v1.json', lambda v: v['obligations'][0].update(status='complete', evidence=['success']))
        self.rejected()

    def test_missing_artifact(self):
        (self.directory / 'sources/specification-v6.md').unlink()
        self.rejected()

    def test_unjustified_exclusion(self):
        self.mutate('inventory-v1.json', lambda v: v['obligations'][0].update(route='excluded', reason=''))
        self.rejected()

    def test_justified_but_unapproved_exclusion(self):
        self.mutate('inventory-v1.json', lambda v: v['obligations'][0].update(route='excluded', reason='Seems optional'))
        self.rejected()

    def test_missing_test(self):
        self.mutate('inventory-v1.json', lambda v: v['obligations'][0].update(proposed_test=''))
        self.rejected()

    def test_parent_escape(self):
        self.mutate('catalog-v1.json', lambda v: v['sources'][0].update(path='../outside'))
        self.rejected()

    def test_symlink(self):
        target = self.directory / 'sources/plan.md'
        content = target.read_bytes()
        target.unlink()
        outside = self.root / 'outside'
        outside.write_bytes(content)
        target.symlink_to(outside)
        self.rejected()

    def test_oversized(self):
        (self.directory / 'inventory-v1.json').write_bytes(b' ' * (checker.MAX_BYTES + 1))
        self.rejected()

    def test_dependency(self):
        self.mutate('inventory-v1.json', lambda v: v['obligations'][0].update(depends_on=['TASK-99']))
        self.rejected()

    def test_missing_reference(self):
        self.mutate('inventory-v1.json', lambda v: v['obligations'][0].update(source_ref='missing:1'))
        self.rejected()

    def test_altered_source_hash(self):
        self.mutate('catalog-v1.json', lambda v: v['sources'][0].update(sha256='0' * 64))
        self.rejected()

    def test_oracle_substitution(self):
        self.mutate('inventory-v1.json', lambda v: v['obligations'][0].update(assertion='All tests passed'))
        self.rejected()

    def test_protocol_success_forgery(self):
        self.mutate('protocols-v1.json', lambda v: v['protocols']['task-01'].update(closure='Already accepted'))
        self.rejected()

    def test_missing_protocol(self):
        self.mutate('protocols-v1.json', lambda v: v['protocols'].pop('task-01'))
        self.rejected()

    def test_removed_inspection_anchor(self):
        self.mutate('catalog-v1.json', lambda v: v['inspection_anchors'].pop())
        self.rejected()

    def test_directory_symlink(self):
        target = self.directory / 'sources'
        outside = self.root / 'outside-sources'
        target.rename(outside)
        target.symlink_to(outside, target_is_directory=True)
        self.rejected()

    def test_invalid_utf8(self):
        (self.directory / 'inventory-v1.json').write_bytes(b'\xff')
        self.rejected()

    def test_policy_promotion(self):
        self.mutate('inventory-v1.json', lambda v: v.update(claim_policy='Production qualified'))
        self.rejected()

    def test_extra_row_success(self):
        self.mutate('inventory-v1.json', lambda v: v['obligations'][0].update(qualification_result='passed'))
        self.rejected()

    def test_extra_top_level_success(self):
        self.mutate('inventory-v1.json', lambda v: v.update(qualification_result='passed'))
        self.rejected()

    def test_boolean_version(self):
        self.mutate('inventory-v1.json', lambda v: v.update(version=True))
        self.rejected()

    def test_duplicate_json_key(self):
        path = self.directory / 'inventory-v1.json'
        text = path.read_text()
        path.write_text(text.replace('"version": 1', '"version": 1, "version": 1', 1))
        self.rejected()

    def test_module_assertion_substitution(self):
        self.mutate('catalog-v1.json', lambda v: v['modules'][0].update(source_assertion='Production qualification passed'))
        self.rejected()

    def test_feature_assertion_substitution(self):
        self.mutate('catalog-v1.json', lambda v: v['features'][0].update(source_assertion='Production qualification passed'))
        self.rejected()

    def test_format_assertion_substitution(self):
        self.mutate('catalog-v1.json', lambda v: v['formats'][0].update(source_assertion='Production qualification passed'))
        self.rejected()

    def test_unhandled_reference_kind(self):
        self.mutate('inventory-v1.json', lambda v: v['obligations'][0].update(source_ref='crates/abi-foundation/src/http.rs:1'))
        self.assertTrue(any('unhandled source reference kind' in e for e in checker.validate(self.root, live=False)))

    def test_no_keyword_obligations_have_direct_cases(self):
        value = json.loads((self.directory / 'inventory-v1.json').read_text())
        rows = {r['id']: r for r in value['obligations']}
        original = (self.directory / 'inventory-v1.json').read_bytes()
        for line in (552, 554, 795, 800, 808):
            row = rows['V6-%04d' % line]
            self.assertEqual('included', row['route'])
            self.assertTrue(all(row['case_contract'][k] for k in ('procedure', 'expected', 'refused')))
            (self.directory / 'inventory-v1.json').write_bytes(original)
            self.mutate('inventory-v1.json', lambda v, rid=row['id']: v['obligations'].__setitem__(slice(None), [r for r in v['obligations'] if r['id'] != rid]))
            self.rejected()

    def test_declared_modules_cannot_close_behavior(self):
        catalog = json.loads((self.directory / 'catalog-v1.json').read_text())
        rows = json.loads((self.directory / 'inventory-v1.json').read_text())['obligations']
        ids = {r['id'] for r in rows}
        for kind in ('modules', 'features', 'formats'):
            for declaration in catalog[kind]:
                self.assertNotIn(declaration['id'], ids)
                self.assertEqual('open', declaration['missing_behavior_blocker']['status'])
        for declaration in catalog['modules']:
            if declaration['source_assertion'] in ('pub mod http;', 'pub mod cluster;'):
                self.assertTrue(declaration['behavior_links'])

    def test_missing_behavior_cannot_be_marked_accepted(self):
        self.mutate('catalog-v1.json', lambda v: v['modules'][0]['missing_behavior_blocker'].update(status='accepted'))
        self.rejected()

    def test_scope_review_removal(self):
        self.mutate('scope-review-v1.json', lambda v: v['decisions'].pop())
        self.rejected()

    def test_task_index_is_inspection_only(self):
        catalog = json.loads((self.directory / 'catalog-v1.json').read_text())
        self.assertNotIn('V6-1563', {r['id'] for r in catalog['items']})
        anchor = next(r for r in catalog['inspection_anchors'] if r['id'] == 'V6-1563')
        self.assertEqual('inspection-only', anchor['classification'])

    def test_excluded_research_and_ui_cannot_gate_task_two(self):
        rows = json.loads((self.directory / 'inventory-v1.json').read_text())['obligations']
        targets = [r for r in rows if r.get('applicability', {}).get('profile') in ('specimen-research', 'explorer-ui')]
        self.assertTrue(targets)
        for row in targets:
            self.assertEqual('excluded', row['route'])
            self.assertNotEqual(2, row['task'])
            self.assertTrue(row['closure']['behavior_remains_open'])

    def test_conditional_profile_cannot_be_erased(self):
        self.mutate('inventory-v1.json', lambda v: next(r for r in v['obligations'] if r.get('applicability', {}).get('profile') == 'affected-substrate-profile-only').pop('applicability'))
        self.rejected()

    def test_direct_refusal_oracle_cannot_be_changed(self):
        self.mutate('inventory-v1.json', lambda v: next(r for r in v['obligations'] if r['id'] == 'V6-0800')['case_contract'].update(refused='Allow stale authority'))
        self.rejected()

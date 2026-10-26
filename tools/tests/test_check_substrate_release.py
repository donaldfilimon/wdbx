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


class LiveRelease(unittest.TestCase):
    """Synthetic trusted identities; never refresh the real catalog pin."""
    def setUp(self):
        from unittest.mock import patch
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.directory = self.root / checker.DIRECTORY
        shutil.copytree(ROOT / checker.DIRECTORY, self.directory)
        self.catalog_path = self.directory / 'catalog-v1.json'
        self.catalog = json.loads(self.catalog_path.read_text())
        for source in self.catalog['sources']:
            if source['live_path']:
                self.write(source['live_path'], (self.directory / source['path']).read_bytes())
        for kind in ('modules', 'features', 'formats'):
            for row in self.catalog[kind]:
                data = ('synthetic declaration ' + row['path']).encode()
                self.write(row['path'], data)
                row['sha256'] = checker.digest(data)
        self.catalog_path.write_text(json.dumps(self.catalog))
        self.pin = patch.object(checker, 'CATALOG_PIN', checker.digest(self.catalog_path.read_bytes()))
        self.pin.start()
        self.addCleanup(self.pin.stop)
        self.path = self.catalog['modules'][0]['path']

    def write(self, path, data):
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)

    def report(self):
        report = {}
        errors = checker.validate(self.root, report=report)
        self.assertEqual(errors, report['errors'])
        self.assertEqual(not errors, report['valid'])
        self.assertEqual(1, report['version'])
        self.assertEqual('live', report['mode'])
        return report

    def test_exact_live_identities(self):
        self.assertEqual([], checker.validate(self.root))
        self.assertEqual([], self.report()['drifted_paths'])
        self.assertTrue(self.report()['complete'])

    def test_changed_grouped_declarations(self):
        self.write(self.path, b'pub mod newly_added;')
        report = self.report()
        row, = report['drifted_paths']
        self.assertEqual(self.path, row['path'])
        self.assertEqual([r['id'] for r in self.catalog['modules'] if r['path'] == self.path], row['declaration_ids'])
        self.assertEqual(checker.digest(b'pub mod newly_added;'), row['actual_sha256'])
        self.assertIsNone(row['read_error'])
        self.assertTrue(report['complete'])
        self.assertEqual([], checker.validate(self.root, live=False))

    def test_read_refusals(self):
        for failure in ('missing', 'oversize', 'symlink', 'unreadable'):
            with self.subTest(failure=failure):
                target = self.root / self.path
                if target.exists() or target.is_symlink():
                    target.unlink()
                if failure == 'oversize':
                    target.write_bytes(b' ' * (checker.MAX_BYTES + 1))
                elif failure == 'symlink':
                    target.symlink_to(self.catalog_path)
                elif failure == 'unreadable':
                    target.mkdir()
                row, = self.report()['drifted_paths']
                self.assertIsNone(row['actual_sha256'])
                self.assertTrue(row['read_error'])
                if target.is_dir() and not target.is_symlink():
                    target.rmdir()

    def test_changed_source(self):
        source = next(r for r in self.catalog['sources'] if r['live_path'])
        self.write(source['live_path'], b'changed')
        row, = self.report()['drifted_paths']
        self.assertEqual([source['id']], row['source_ids'])
        self.assertEqual([], row['declaration_ids'])

    def test_cache_unique_live_reads(self):
        from unittest.mock import patch
        original = checker.read
        with patch.object(checker, 'read', wraps=original) as observed:
            self.assertEqual([], checker.validate(self.root))
        self.assertEqual(1, sum(c.args[1] == self.path for c in observed.call_args_list))

    def test_untrusted_and_malformed_catalog(self):
        for data in (b'{', b'{"version": 1}', self.catalog_path.read_bytes() + b' '):
            self.catalog_path.write_bytes(data)
            report = self.report()
            self.assertFalse(report['complete'])
            self.assertTrue(report['errors'])
            self.assertEqual([], report['drifted_paths'])

    def test_conflicting_expected_hashes(self):
        from unittest.mock import patch
        self.catalog['modules'][1]['sha256'] = '0' * 64
        self.catalog_path.write_text(json.dumps(self.catalog))
        with patch.object(checker, 'CATALOG_PIN', checker.digest(self.catalog_path.read_bytes())):
            report = self.report()
        self.assertFalse(report['complete'])
        self.assertTrue(any('conflicting' in e for e in report['errors']))

    def test_cli_json_and_default_refuse(self):
        import contextlib
        import io
        from unittest.mock import patch
        self.write(self.path, b'changed')
        for flag in ([], ['--drift-report']):
            stdout, stderr = io.StringIO(), io.StringIO()
            with patch('sys.argv', ['checker', '--root', str(self.root)] + flag), contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                self.assertEqual(1, checker.main())
            if flag:
                self.assertFalse(json.loads(stdout.getvalue())['valid'])
                self.assertIn('FAIL:', stderr.getvalue())
            else:
                self.assertIn('FAIL:', stderr.getvalue())
                self.assertEqual('', stdout.getvalue())

    def test_cli_modes_mutually_exclusive(self):
        import contextlib
        import io
        from unittest.mock import patch
        with patch('sys.argv', ['checker', '--snapshot-only', '--drift-report']), contextlib.redirect_stderr(io.StringIO()):
            with self.assertRaises(SystemExit) as raised:
                checker.main()
        self.assertEqual(2, raised.exception.code)

    def test_two_paths_group_module_and_format_rows(self):
        module_path = 'crates/abi-wdbx/src/lib.rs'
        format_path = 'crates/abi-wdbx/src/v2.rs'
        self.write(module_path, b'pub mod model;')
        self.write(format_path, b'pub fn read_v2_root() {}')
        rows = {r['path']: r for r in self.report()['drifted_paths']}
        self.assertEqual({module_path, format_path}, set(rows))
        self.assertEqual(38, len(rows[module_path]['declaration_ids']))
        self.assertEqual(1, len(rows[format_path]['declaration_ids']))

    def test_missing_source_and_os_read_error(self):
        from unittest.mock import patch
        source = next(r for r in self.catalog['sources'] if r['live_path'])
        (self.root / source['live_path']).unlink()
        row, = self.report()['drifted_paths']
        self.assertEqual([source['id']], row['source_ids'])
        self.assertIsNone(row['actual_sha256'])
        original = checker.read
        def refuse(root, path):
            if path == source['live_path']:
                raise PermissionError('synthetic permission refusal')
            return original(root, path)
        with patch.object(checker, 'read', side_effect=refuse):
            row, = self.report()['drifted_paths']
        self.assertIn('permission refusal', row['read_error'])
        self.assertIsNone(row['actual_sha256'])

    def test_pinned_malformed_catalog_is_incomplete(self):
        from unittest.mock import patch
        self.catalog_path.write_bytes(b'{"version": 1}')
        with patch.object(checker, 'CATALOG_PIN', checker.digest(self.catalog_path.read_bytes())):
            report = self.report()
        self.assertFalse(report['complete'])
        self.assertFalse(report['valid'])
        self.assertEqual([], report['drifted_paths'])

    def test_success_json_has_no_prose(self):
        import contextlib
        import io
        from unittest.mock import patch
        stdout = io.StringIO()
        with patch('sys.argv', ['checker', '--root', str(self.root), '--drift-report']), contextlib.redirect_stdout(stdout):
            self.assertEqual(0, checker.main())
        self.assertTrue(json.loads(stdout.getvalue())['valid'])

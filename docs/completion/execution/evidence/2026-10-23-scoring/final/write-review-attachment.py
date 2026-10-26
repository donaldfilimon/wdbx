"""Attach a completed bounded review without modifying C0 requirements."""
import hashlib,json
from pathlib import Path
folder=Path(__file__).resolve().parent
root=folder.parents[5]
packet=folder.parent
out=folder/'review-attachment.json'
assert not out.exists()
def ref(p):
 assert p.is_file() and not p.is_symlink()
 return {'path':p.relative_to(root).as_posix(),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}
mapping=packet/'evidence/requirement-mapping.json'
m=json.loads(mapping.read_text())
register=json.loads((root/'docs/completion/execution/register-v1.json').read_text())['requirements']
byid={r['id']:r for r in register}
ids=sorted(set(m['cases'])|set(m['program_chapter_routes']))
rows=[]
for i in ids:
 r=byid[i]
 assert (r['status'],r['evidence_level'],r['test_result'])==('open','C0','unknown')
 rows.append({'id':i,'status':'open','evidence_level':'C0','test_result':'unknown','scope':'Independent review of current bounded local mechanisms only; whole frozen requirement remains unqualified.'})
review=folder/'task-8-scoring-final-review.md'
assert review.is_file(), 'save completed final review first'
reviews=[ref(review)]
rereview=folder/'task-8-scoring-final-fix-rereview.md'
if rereview.exists():reviews.append(ref(rereview))
out.write_text(json.dumps({'kind':'C0-only partial-mechanism review attachment; no requirement promotion','reviews':reviews,'source_freeze':ref(folder/'product-freeze.json'),'mapping':ref(mapping),'requirements':rows,'decisions':[{'id':i,'scope':'Bounded partial design mechanism only; unmet research and host clauses remain open.'} for i in sorted(m['decisions'])],'limitations':'No ABI owner approval, engine adoption, efficacy, independent contamination/adjudication, real host fencing, physical memory/concurrency, installed/platform or elapsed-operation qualification is inferred.'},indent=2)+'\n')
print(json.dumps({'requirements':len(rows),'decisions':len(m['decisions']),'attachment':str(out)}))

"""Regenerate the reader from its authoritative Markdown (pip install markdown-it-py)."""
import json
import re
from pathlib import Path
from markdown_it import MarkdownIt

root = Path(__file__).resolve().parents[1]
source = (root / 'public/WDBX-Specimen-Architecture-Specification.md').read_text()
parts = re.split(r'^## (\d+)\. (.+)\n', source, flags=re.M)
renderer = MarkdownIt('commonmark', {'html': False}).enable(['table', 'strikethrough'])
chapters = []
for index in range(1, len(parts), 3):
    body = parts[index + 2]
    chapters.append({'number': int(parts[index]), 'title': parts[index + 1], 'text': body,
                     'html': renderer.render(body), 'words': len(body.split())})
assert [chapter['number'] for chapter in chapters] == list(range(1, 27))
assert all(chapter['html'].strip() and chapter['words'] > 0 for chapter in chapters)
(root / 'lib/specification.json').write_text(json.dumps(chapters, ensure_ascii=False, separators=(',', ':')))
print('Updated all 26 reader chapters, rendered content and reading times.')

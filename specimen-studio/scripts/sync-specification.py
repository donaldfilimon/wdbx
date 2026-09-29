"""Regenerate the reader from its authoritative Markdown (pip install markdown-it-py).

Each chapter becomes prose `html` segments and `figure` segments, one per
top-level fenced block (as markdown-it parses it), with a stable id
`ch<N>-<k>` the reader's diagram registry keys on. Rendering the segments one
by one must produce exactly the HTML of rendering the chapter whole, so the
split can never change how the prose reads.
"""
import json
import re
from pathlib import Path
from markdown_it import MarkdownIt

root = Path(__file__).resolve().parents[1]
source = (root / 'public/WDBX-Specimen-Architecture-Specification.md').read_text()
parts = re.split(r'^## (\d+)\. (.+)\n', source, flags=re.M)
renderer = MarkdownIt('commonmark', {'html': False}).enable(['table', 'strikethrough'])
chapters = []
figures = 0
for index in range(1, len(parts), 3):
    number, body = int(parts[index]), parts[index + 2]
    lines = body.splitlines(keepends=True)
    fences = [t for t in renderer.parse(body) if t.type == 'fence' and t.level == 0]
    segments, pieces, cursor = [], [], 0
    for k, token in enumerate(fences, start=1):
        start, end = token.map
        prose = ''.join(lines[cursor:start])
        if prose.strip():
            segments.append({'kind': 'html', 'html': renderer.render(prose)})
            pieces.append(prose)
        segments.append({'kind': 'figure', 'id': f'ch{number}-{k}',
                         'lang': (token.info.split() or ['text'])[0],
                         'code': token.content})
        pieces.append(''.join(lines[start:end]))
        cursor = end
        figures += 1
    tail = ''.join(lines[cursor:])
    if tail.strip():
        segments.append({'kind': 'html', 'html': renderer.render(tail)})
        pieces.append(tail)
    assert ''.join(renderer.render(p) for p in pieces) == renderer.render(body), (
        f'chapter {number}: splitting at figures changes its rendering')
    chapters.append({'number': number, 'title': parts[index + 1], 'text': body,
                     'segments': segments, 'words': len(body.split())})
assert [chapter['number'] for chapter in chapters] == list(range(1, 27))
assert all(chapter['segments'] and chapter['words'] > 0 for chapter in chapters)
assert figures == 29, figures
(root / 'lib/specification.json').write_text(json.dumps(chapters, ensure_ascii=False, separators=(',', ':')))
print(f'Updated all 26 reader chapters ({figures} figures), rendered content and reading times.')

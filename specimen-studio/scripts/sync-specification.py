"""Regenerate the reader from its authoritative Markdown (pip install markdown-it-py).

Each chapter becomes prose `html` segments and `figure` segments, one per
top-level fenced block, with a stable id `ch<N>-<k>` the reader's diagram
registry keys on.
"""
import json
import re
from pathlib import Path
from markdown_it import MarkdownIt

root = Path(__file__).resolve().parents[1]
source = (root / 'public/WDBX-Specimen-Architecture-Specification.md').read_text()
parts = re.split(r'^## (\d+)\. (.+)\n', source, flags=re.M)
renderer = MarkdownIt('commonmark', {'html': False}).enable(['table', 'strikethrough'])
fence = re.compile(r'^```(\w*)\n(.*?)^```[ \t]*\n?', flags=re.M | re.S)
chapters = []
figures = 0
for index in range(1, len(parts), 3):
    number, body = int(parts[index]), parts[index + 2]
    segments, cursor, rebuilt = [], 0, ''
    for k, match in enumerate(fence.finditer(body), start=1):
        prose = body[cursor:match.start()]
        if prose.strip():
            segments.append({'kind': 'html', 'html': renderer.render(prose)})
        segments.append({'kind': 'figure', 'id': f'ch{number}-{k}',
                         'lang': match.group(1) or 'text', 'code': match.group(2)})
        rebuilt += prose + match.group(0)
        cursor = match.end()
        figures += 1
    tail = body[cursor:]
    if tail.strip():
        segments.append({'kind': 'html', 'html': renderer.render(tail)})
    assert rebuilt + tail == body, number
    chapters.append({'number': number, 'title': parts[index + 1], 'text': body,
                     'segments': segments, 'words': len(body.split())})
assert [chapter['number'] for chapter in chapters] == list(range(1, 27))
assert all(chapter['segments'] and chapter['words'] > 0 for chapter in chapters)
assert figures == 29, figures
(root / 'lib/specification.json').write_text(json.dumps(chapters, ensure_ascii=False, separators=(',', ':')))
print(f'Updated all 26 reader chapters ({figures} figures), rendered content and reading times.')

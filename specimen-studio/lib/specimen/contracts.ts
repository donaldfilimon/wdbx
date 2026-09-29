import type { Specimen } from './types';
export function validateCollections(s: Specimen): void {
  const fail = (message: string): never => {
    throw new Error(`Invalid specimen: ${message}.`);
  };
  const str = (v: unknown, max = 8000): v is string =>
    typeof v === 'string' && v.length <= max;
  const id = (v: unknown) => str(v, 200) && v.length > 0;
  const date = (v: unknown) => str(v, 100) && Number.isFinite(Date.parse(v));
  const list = (v: unknown, max = 100000): v is unknown[] =>
    Array.isArray(v) && v.length <= max;
  const ids = (v: unknown) => list(v, 10000) && v.every((x) => id(x));
  const unique = (rows: { id?: string; ref?: string }[]) =>
    new Set(rows.map((r) => r.id ?? r.ref)).size === rows.length;
  if (!id(s.name) || !date(s.updatedAt)) fail('name or update date');
  if (!Number.isFinite(s.atp.lastUpdate) || s.atp.lastUpdate < 0)
    fail('ATP timestamp');
  if (
    s.events.length > 500 ||
    s.mutations.length > 100000 ||
    s.proposals.length > 10000 ||
    s.attachments.length > 50000
  )
    fail('collection size');
  for (const rows of [
    s.nodes,
    s.resources,
    s.attachments,
    s.history,
    s.events,
    s.mutations,
    s.proposals,
  ])
    if (rows.some((r) => !r || typeof r !== 'object') || !unique(rows))
      fail('duplicate or missing record');
  const entries = new Set<string>(),
    alternatives = new Set<string>();
  for (const n of s.nodes) {
    if (
      !id(n.ref) ||
      !str(n.name, 100) ||
      !str(n.contextId, 2000) ||
      typeof n.jitter !== 'boolean' ||
      !['low', 'medium', 'high'].includes(n.resolution) ||
      !['neutral', 'warm', 'curious', 'cautious'].includes(n.tone) ||
      !date(n.createdAt) ||
      !list(n.entries, 100)
    )
      fail('node metadata');
    for (const e of n.entries) {
      if (!e || !id(e.id) || entries.has(e.id) || !list(e.alternatives, 100))
        fail('entry identity');
      entries.add(e.id);
      for (const a of e.alternatives) {
        if (
          !a ||
          !id(a.id) ||
          alternatives.has(a.id) ||
          !str(a.inhibition, 2000) ||
          !str(a.action, 8000)
        )
          fail('vote identity or content');
        alternatives.add(a.id);
      }
    }
  }
  for (const r of s.resources)
    if (
      !id(r.ref) ||
      !str(r.text, 2000) ||
      !str(r.value, 12000) ||
      !str(r.resourceId, 200)
    )
      fail('memory metadata');
  for (const a of s.attachments)
    if (
      !id(a.id) ||
      typeof a.hard !== 'boolean' ||
      typeof a.bidirectional !== 'boolean'
    )
      fail('attachment flags');
  if (
    s.history.filter((h) => h.pinned).length > s.settings.pinLimit ||
    s.history.filter((h) => !h.pinned).length > s.settings.historyLimit
  )
    fail('history retention limits');
  for (const h of s.history) {
    if (
      !id(h.id) ||
      !date(h.createdAt) ||
      !str(h.input) ||
      typeof h.pinned !== 'boolean' ||
      !Number.isFinite(h.duration) ||
      h.duration < 0 ||
      !Number.isFinite(h.supervisedUntil) ||
      !list(h.segments, 10000) ||
      !list(h.votes, 10000) ||
      !list(h.trace, 1000) ||
      !list(h.feedback, 10000) ||
      !h.feedback.every((f) => str(f, 200))
    )
      fail('conversation metadata');
    for (const seg of h.segments)
      if (
        !seg ||
        !id(seg.id) ||
        !str(seg.text, 20000) ||
        !ids(seg.contributors) ||
        !ids(seg.sourceVotes) ||
        !list(seg.transformations, 100) ||
        !seg.transformations.every((t) => str(t, 1000)) ||
        !['green', 'violet', 'amber', 'blue', 'rose', 'cyan'].includes(
          seg.color,
        )
      )
        fail('output segment');
    for (const v of h.votes)
      if (
        !v ||
        !id(v.id) ||
        !id(v.nodeRef) ||
        !id(v.entryRef) ||
        !str(v.action) ||
        !str(v.input) ||
        !Number.isFinite(v.confidence) ||
        Math.abs(v.confidence) > 100 ||
        !Number.isFinite(v.base) ||
        Math.abs(v.base) > 100 ||
        !Number.isFinite(v.strength) ||
        !ids(v.resources)
      )
        fail('recorded vote');
    for (const t of h.trace)
      if (!t || !id(t.id) || !str(t.phase, 200) || !str(t.detail))
        fail('trace entry');
    if (h.visual) {
      const v = h.visual;
      if (
        !v.size ||
        !v.position ||
        ![v.size.width, v.size.height, v.position.x, v.position.y].every(
          Number.isFinite,
        ) ||
        v.size.width <= 0 ||
        v.size.height <= 0 ||
        v.size.width > 10000 ||
        v.size.height > 10000 ||
        !list(v.colorArray, 128) ||
        !v.colorArray.every(
          (c) => typeof c === 'string' && /^#[0-9a-f]{6}$/i.test(c),
        ) ||
        !list(v.brightnessArray, 128) ||
        !v.brightnessArray.every(
          (n) =>
            typeof n === 'number' && Number.isFinite(n) && n >= 0 && n <= 1,
        )
      )
        fail('visual composition');
    }
  }
  for (const e of s.events)
    if (
      !id(e.id) ||
      !str(e.title, 500) ||
      !str(e.type, 100) ||
      !str(e.detail, 20000) ||
      !date(e.createdAt)
    )
      fail('activity record');
  for (const m of s.mutations)
    if (
      !id(m.id) ||
      !id(m.recipient) ||
      !id(m.donor) ||
      !id(m.slot) ||
      !str(m.originalPattern, 2000) ||
      !str(m.originalAction) ||
      !date(m.createdAt)
    )
      fail('mutation lineage');
  for (const p of s.proposals)
    if (
      !id(p.id) ||
      !str(p.pattern) ||
      !Number.isInteger(p.evidence) ||
      p.evidence < 1 ||
      !['pending', 'learned', 'dismissed'].includes(p.status)
    )
      fail('learning proposal');
}

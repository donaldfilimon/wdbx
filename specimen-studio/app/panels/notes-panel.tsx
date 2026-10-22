import { useId, useState, type ReactNode } from 'react';
import { Panel, WButton } from '@/components/wdbx';
import { control, Field } from '@/components/wdbx/form';
import {
  applyCommand,
  querySpecimen,
  type NoteBlock,
  type NoteHit,
  type NoteInline,
  type QueryAnswer,
} from '@/lib/specimen/kernel';
import type { Note, Specimen, SpecimenNode } from '@/lib/specimen/types';

type Save = {
  state: Specimen;
  commit: (next: Specimen) => void;
  pushUndo: (undo: ReturnType<typeof applyCommand>['undo']) => void;
  announce: (message: string) => void;
  act: (fn: () => void) => void;
};

function ask(state: Specimen, query: Parameters<typeof querySpecimen>[1]) {
  try {
    return querySpecimen(state, query);
  } catch {
    return null;
  }
}

export function NotesView({
  state,
  commit,
  pushUndo,
  announce,
  act,
}: {
  state: Specimen;
  commit: (next: Specimen) => void;
  pushUndo: Save['pushUndo'];
  announce: (message: string) => void;
  act: (fn: () => void) => void;
}) {
  const notes = state.notes ?? [];
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const selected = notes.find((note) => note.id === selectedId);
  const backlinks =
    selected &&
    ask(state, { query: 'backlinks', target: `note:${selected.title}` });
  const outgoing =
    selected && ask(state, { query: 'outgoing', note: selected.id });
  const rendered = selected && ask(state, { query: 'render', id: selected.id });
  const graph = ask(state, { query: 'graph', depth: 0 });
  return (
    <div className="grid items-start gap-4 xl:grid-cols-[16rem_minmax(0,1fr)_18rem]">
      <Panel title="Library" description="Titles in this specimen.">
        <div className="grid gap-2 p-3">
          <WButton variant="outline" onClick={() => setSelectedId(null)}>
            New note
          </WButton>
          {notes.length === 0 ? (
            <p className="m-0 text-sm text-muted-foreground">No notes yet.</p>
          ) : (
            <ul className="m-0 grid list-none gap-1 p-0" aria-label="Notes">
              {notes.map((note) => (
                <li key={note.id}>
                  <button
                    type="button"
                    className="min-h-11 w-full rounded-lg px-3 text-left text-sm text-ink hover:bg-surface-2 focus-visible:outline-2 focus-visible:outline-focus"
                    aria-pressed={note.id === selected?.id}
                    onClick={() => setSelectedId(note.id)}
                  >
                    {note.title}
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      </Panel>
      <Panel
        title="Editor"
        description="Wiki links use [[Title]] or [[node:ref|label]]."
      >
        <NoteEditor
          key={`${selected?.id ?? 'new'}:${selected?.updatedAt ?? ''}`}
          note={selected}
          nodes={state.nodes}
          save={{ state, commit, pushUndo, announce, act }}
          onSelect={setSelectedId}
          rendered={rendered && rendered.kind === 'document' ? rendered : null}
        />
      </Panel>
      <div className="grid gap-4">
        <Panel title="Links">
          <div className="grid gap-3 p-4 text-sm">
            <section aria-label="Backlinks">
              <h3 className="m-0 text-xs font-semibold text-ink">Backlinks</h3>
              <HitList
                hits={
                  backlinks && backlinks.kind === 'notes' ? backlinks.notes : []
                }
                empty="No notes link here."
                onOpen={setSelectedId}
              />
            </section>
            <section aria-label="Outgoing links">
              <h3 className="m-0 text-xs font-semibold text-ink">
                Outgoing links
              </h3>
              {outgoing &&
              outgoing.kind === 'links' &&
              outgoing.links.length ? (
                <ul className="m-0 mt-1 list-none p-0">
                  {outgoing.links.map((link, index) => (
                    <li key={`${link.label}-${index}`} className="py-1">
                      {link.label}
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="m-0 mt-1 text-muted-foreground">
                  No outgoing links.
                </p>
              )}
            </section>
          </div>
        </Panel>
        <Panel title="Graph" description="Positions come from the kernel.">
          <NoteGraph
            graph={graph && graph.kind === 'graph' ? graph : null}
            onOpen={setSelectedId}
          />
        </Panel>
      </div>
    </div>
  );
}

function HitList({
  hits,
  empty,
  onOpen,
}: {
  hits: NoteHit[];
  empty: string;
  onOpen: (id: string) => void;
}) {
  if (!hits.length)
    return <p className="m-0 mt-1 text-muted-foreground">{empty}</p>;
  return (
    <ul className="m-0 mt-1 list-none p-0">
      {hits.map((hit) => (
        <li key={hit.id}>
          <button
            type="button"
            className="min-h-11 text-left text-sm text-ink underline-offset-2 hover:underline focus-visible:outline-2 focus-visible:outline-focus"
            onClick={() => onOpen(hit.id)}
          >
            {hit.title}
          </button>
        </li>
      ))}
    </ul>
  );
}

function NoteEditor({
  note,
  nodes,
  save,
  onSelect,
  rendered,
}: {
  note?: Note;
  nodes: SpecimenNode[];
  save: Save;
  onSelect: (id: string | null) => void;
  rendered: Extract<QueryAnswer, { kind: 'document' }> | null;
}) {
  const titleId = useId();
  const bodyId = useId();
  const tagsId = useId();
  const linkId = useId();
  const [title, setTitle] = useState(note?.title ?? '');
  const [body, setBody] = useState(note?.body ?? '');
  const [tags, setTags] = useState((note?.tags ?? []).join(', '));
  const [force, setForce] = useState(false);
  const tagList = tags
    .split(',')
    .map((tag) => tag.trim())
    .filter(Boolean);
  const onCreate = (nextTitle: string) => {
    save.act(() => {
      const applied = applyCommand(save.state, {
        command: 'createNote',
        title: nextTitle,
        body: '',
        tags: [],
      });
      save.pushUndo(applied.undo);
      save.commit(applied.state);
      save.announce(applied.message);
      const created = applied.state.notes?.find(
        (item) => item.title === nextTitle,
      );
      if (created) onSelect(created.id);
    });
  };
  return (
    <form
      className="grid gap-3 p-4"
      onSubmit={(event) => {
        event.preventDefault();
        save.act(() => {
          let next = save.state;
          if (!note) {
            const applied = applyCommand(next, {
              command: 'createNote',
              title,
              body,
              tags: tagList,
            });
            save.pushUndo(applied.undo);
            next = applied.state;
            save.commit(next);
            save.announce(applied.message);
            const created = next.notes?.find(
              (item) => item.title.trim() === title.trim(),
            );
            if (created) onSelect(created.id);
            return;
          }
          if (title.trim() !== note.title) {
            const applied = applyCommand(next, {
              command: 'renameNote',
              id: note.id,
              title,
            });
            save.pushUndo(applied.undo);
            next = applied.state;
          }
          if (body !== note.body) {
            const applied = applyCommand(next, {
              command: 'editNote',
              id: note.id,
              body,
            });
            save.pushUndo(applied.undo);
            next = applied.state;
          }
          const sameTags =
            tagList.length === note.tags.length &&
            tagList.every((tag, index) => tag === note.tags[index]);
          if (!sameTags) {
            const applied = applyCommand(next, {
              command: 'tagNote',
              id: note.id,
              tags: tagList,
            });
            save.pushUndo(applied.undo);
            next = applied.state;
          }
          save.commit(next);
          save.announce('Note saved.');
        });
      }}
    >
      <Field label="Title" htmlFor={titleId}>
        <input
          id={titleId}
          className={control}
          value={title}
          maxLength={200}
          onChange={(event) => setTitle(event.target.value)}
        />
      </Field>
      <Field label="Body" htmlFor={bodyId}>
        <textarea
          id={bodyId}
          className={`${control} min-h-40`}
          value={body}
          maxLength={20000}
          onChange={(event) => setBody(event.target.value)}
        />
      </Field>
      <Field label="Tags" htmlFor={tagsId} hint="Separate tags with commas.">
        <input
          id={tagsId}
          className={control}
          value={tags}
          onChange={(event) => setTags(event.target.value)}
        />
      </Field>
      <Field label="Link a node" htmlFor={linkId}>
        <select
          id={linkId}
          className={control}
          defaultValue=""
          onChange={(event) => {
            const ref = event.target.value;
            event.target.value = '';
            const node = nodes.find((item) => item.ref === ref);
            if (!node) return;
            setBody((current) => {
              const glue = current && !current.endsWith('\n') ? '\n' : '';
              return `${current}${glue}[[node:${node.ref}|${node.name}]]`;
            });
          }}
        >
          <option value="">Choose a node</option>
          {nodes.map((node) => (
            <option key={node.ref} value={node.ref}>
              {node.name}
            </option>
          ))}
        </select>
      </Field>
      <div className="flex flex-wrap gap-2">
        <WButton type="submit" variant="primary">
          Save note
        </WButton>
        {note && (
          <WButton
            type="button"
            variant="outline"
            onClick={() =>
              save.act(() => {
                const applied = applyCommand(save.state, {
                  command: 'deleteNote',
                  id: note.id,
                  force,
                });
                save.pushUndo(applied.undo);
                save.commit(applied.state);
                save.announce(applied.message);
                onSelect(null);
              })
            }
          >
            Delete note
          </WButton>
        )}
      </div>
      {note && (
        <label className="flex min-h-11 items-center gap-2 text-sm text-ink">
          <input
            type="checkbox"
            className="size-4 accent-teal"
            checked={force}
            onChange={(event) => setForce(event.target.checked)}
          />
          Delete even if other notes link here
        </label>
      )}
      {rendered && (
        <div className="grid gap-2 border-t border-line pt-3 text-sm text-ink">
          <Blocks blocks={rendered.blocks} onCreate={onCreate} />
        </div>
      )}
    </form>
  );
}

function NoteHeading({
  level,
  children,
}: {
  level: number;
  children: ReactNode;
}) {
  const className = 'm-0 font-semibold';
  if (level <= 2) return <h2 className={className}>{children}</h2>;
  if (level === 3) return <h3 className={className}>{children}</h3>;
  if (level === 4) return <h4 className={className}>{children}</h4>;
  if (level === 5) return <h5 className={className}>{children}</h5>;
  return <h6 className={className}>{children}</h6>;
}

function Blocks({
  blocks,
  onCreate,
}: {
  blocks: NoteBlock[];
  onCreate: (title: string) => void;
}) {
  return blocks.map((block, index) => {
    const key = `${block.type}-${index}`;
    if (block.type === 'heading')
      return (
        <NoteHeading key={key} level={block.level}>
          <Inlines inlines={block.inlines} onCreate={onCreate} />
        </NoteHeading>
      );
    if (block.type === 'code')
      return (
        <pre
          key={key}
          className="m-0 overflow-auto rounded-lg bg-surface-2 p-3"
        >
          {block.text}
        </pre>
      );
    if (block.type === 'rule') return <hr key={key} className="border-line" />;
    if (block.type === 'list')
      return (
        <ul key={key} className="m-0 list-disc pl-5">
          {block.items.map((item, itemIndex) => (
            <li key={itemIndex}>
              <Inlines inlines={item} onCreate={onCreate} />
            </li>
          ))}
        </ul>
      );
    return (
      <p key={key} className="m-0">
        <Inlines inlines={block.inlines} onCreate={onCreate} />
      </p>
    );
  });
}

function Inlines({
  inlines,
  onCreate,
}: {
  inlines: NoteInline[];
  onCreate: (title: string) => void;
}) {
  return inlines.map((inline, index) => {
    const key = `${inline.type}-${index}`;
    if (inline.type === 'text' || inline.type === 'code')
      return <span key={key}>{inline.text}</span>;
    if (inline.type === 'break') return <br key={key} />;
    if (inline.type === 'em')
      return (
        <em key={key}>
          <Inlines inlines={inline.inlines} onCreate={onCreate} />
        </em>
      );
    if (inline.type === 'strong')
      return (
        <strong key={key}>
          <Inlines inlines={inline.inlines} onCreate={onCreate} />
        </strong>
      );
    if (inline.kind === 'note' && !inline.resolved)
      return (
        <button
          key={key}
          type="button"
          className="min-h-11 text-danger underline-offset-2 hover:underline focus-visible:outline-2 focus-visible:outline-focus"
          onClick={() => onCreate(inline.target)}
        >
          Create note {inline.label}
        </button>
      );
    return (
      <span key={key} className={inline.resolved ? 'text-ink' : 'text-danger'}>
        {inline.label}
      </span>
    );
  });
}

function NoteGraph({
  graph,
  onOpen,
}: {
  graph: Extract<QueryAnswer, { kind: 'graph' }> | null;
  onOpen: (id: string) => void;
}) {
  const nodes = graph?.nodes ?? [];
  const byId = new Map(nodes.map((node) => [node.id, node]));
  return (
    <div className="grid gap-3 p-4">
      <svg
        viewBox="-240 -240 480 480"
        className="h-56 w-full"
        aria-hidden="true"
      >
        {(graph?.edges ?? []).map((edge) => {
          const from = byId.get(edge.from);
          const to = byId.get(edge.to);
          if (!from || !to) return null;
          return (
            <line
              key={`${edge.from}-${edge.to}`}
              x1={from.x}
              y1={from.y}
              x2={to.x}
              y2={to.y}
              className="stroke-line"
              strokeWidth="2"
            />
          );
        })}
        {nodes.map((node) => (
          <circle
            key={node.id}
            cx={node.x}
            cy={node.y}
            r="8"
            className="fill-teal"
          />
        ))}
      </svg>
      <ul className="m-0 list-none p-0" aria-label="Note graph">
        {nodes.length === 0 ? (
          <li className="text-sm text-muted-foreground">No notes to draw.</li>
        ) : (
          nodes.map((node) => (
            <li key={node.id}>
              <button
                type="button"
                className="min-h-11 text-left text-sm text-ink focus-visible:outline-2 focus-visible:outline-focus"
                onClick={() => onOpen(node.id)}
              >
                {node.title}
              </button>
            </li>
          ))
        )}
      </ul>
    </div>
  );
}

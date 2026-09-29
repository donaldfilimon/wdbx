import { useId, useState } from 'react';
import { Download, Play, Trash2 } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { EmptyState, Panel, WButton } from '@/components/wdbx';
import {
  generate,
  installModel,
  loadModel,
  removeModel,
  unloadModel,
  type ModelStatus,
} from '@/lib/specimen/lab-api';
import { persistNative, pickNativeFile } from '@/lib/specimen/native';
import type { Specimen } from '@/lib/specimen/types';
import type { Lab } from './use-lab';

const mb = (bytes?: number) =>
  bytes ? `${(bytes / 1024 / 1024).toFixed(0)} MB` : 'size unknown';

/** Installed and available local models, with install, load and remove. */
export function ModelsPanel({ lab }: { lab: Lab }) {
  const busy = !!lab.busy;
  if (!lab.models)
    return (
      <EmptyState
        title="Reading models"
        text="The model store is busy or still answering; this refreshes every few seconds."
      />
    );
  return (
    <Panel
      title="Local models"
      description="Pinned sources, verified by size and SHA-256 on install."
    >
      <ul className="m-0 grid list-none gap-3 p-4">
        {lab.models.map(({ model, installed, loaded }) => (
          <li
            key={model.id}
            className="model-card grid gap-2 rounded-lg border border-line bg-surface-2 p-3 text-sm"
          >
            <div className="flex flex-wrap items-center gap-2">
              <strong className="text-ink">{model.name}</strong>
              <Badge variant="outline">{model.kind}</Badge>
              <span className="text-xs text-ink-soft">
                {loaded ? 'Loaded' : installed ? 'Installed' : 'Not installed'}{' '}
                · {mb(model.size)}
              </span>
            </div>
            {model.license && (
              <p className="m-0 text-xs text-ink-soft">{model.license}</p>
            )}
            {model.memoryBytes ? (
              <p className="m-0 text-xs text-ink-soft">
                About {(model.memoryBytes / 1e9).toFixed(1)} GB of memory when
                loaded
              </p>
            ) : null}
            <details className="text-xs">
              <summary className="min-h-11 cursor-pointer content-center font-semibold text-ink-soft">
                Source and verification
              </summary>
              <div className="grid gap-1 break-all">
                {model.source && (
                  <a
                    href={model.source}
                    target="_blank"
                    rel="noreferrer"
                    className="text-teal-strong"
                  >
                    Publisher model card
                  </a>
                )}
                <code>{model.revision}</code>
                <code>{model.sha256}</code>
              </div>
            </details>
            <div className="flex flex-wrap gap-2">
              {!installed ? (
                <>
                  <WButton
                    variant="outline"
                    disabled={busy}
                    onClick={() =>
                      void lab.work(`Installing ${model.name}`, (jobId) =>
                        installModel(model.id, jobId, lab.progress).then(
                          () => {},
                        ),
                      )
                    }
                  >
                    <Download aria-hidden="true" size={14} />
                    Download
                  </WButton>
                  <WButton
                    variant="outline"
                    disabled={busy}
                    onClick={() =>
                      void lab.work(
                        `Importing ${model.name}`,
                        async (jobId) => {
                          const path = await pickNativeFile();
                          if (path)
                            await installModel(
                              model.id,
                              jobId,
                              lab.progress,
                              path,
                            );
                        },
                      )
                    }
                  >
                    Import file
                  </WButton>
                </>
              ) : (
                <WButton
                  variant="outline"
                  disabled={busy}
                  onClick={() =>
                    void lab.work(`Removing ${model.name}`, () =>
                      removeModel(model.id).then(() => {}),
                    )
                  }
                >
                  <Trash2 aria-hidden="true" size={14} />
                  Remove
                </WButton>
              )}
              {installed && !loaded && model.kind === 'text' && (
                <WButton
                  variant="outline"
                  disabled={busy}
                  onClick={() =>
                    void lab.work(`Loading ${model.name}`, (jobId) =>
                      loadModel(model.id, jobId, lab.progress).then(() => {}),
                    )
                  }
                >
                  Load
                </WButton>
              )}
              {loaded && (
                <WButton
                  variant="outline"
                  disabled={busy}
                  onClick={() =>
                    void lab.work('Unloading model', () =>
                      unloadModel().then(() => {}),
                    )
                  }
                >
                  Unload
                </WButton>
              )}
            </div>
          </li>
        ))}
      </ul>
    </Panel>
  );
}

const generative = (models: ModelStatus[] | null) =>
  (models ?? []).filter(
    (m) => m.model.kind === 'text' || m.model.kind === 'image',
  );

/** Compose with an installed local model; learning stays explicit. */
export function GeneratePanel({
  lab,
  specimen,
}: {
  lab: Lab;
  specimen: Specimen;
}) {
  const id = useId();
  const options = generative(lab.models);
  const [picked, setPicked] = useState('');
  const modelId = picked || options[0]?.model.id || '';
  const [prompt, setPrompt] = useState('');
  const [seed, setSeed] = useState('104729');
  const seedValue = Math.max(
    0,
    Math.min(2147483647, Math.trunc(Number(seed)) || 0),
  );
  const installed =
    options.find((m) => m.model.id === modelId)?.installed ?? false;
  return (
    <Panel
      title="Local generation"
      description="Runs on this machine with the bundled runtimes."
    >
      <form
        className="grid gap-3 p-4"
        onSubmit={(e) => {
          e.preventDefault();
          void lab.work('Generating locally', async (jobId) => {
            await persistNative(specimen);
            lab.applySnapshot(
              (await generate(modelId, prompt, seedValue, jobId, lab.progress))
                .snapshot,
            );
          });
        }}
      >
        <label
          htmlFor={`${id}-model`}
          className="grid gap-1 text-xs font-semibold text-ink"
        >
          Model
          <select
            id={`${id}-model`}
            value={modelId}
            onChange={(e) => setPicked(e.target.value)}
            className="min-h-11 rounded-lg border border-line bg-surface-2 px-2"
          >
            {options.map(({ model, installed: ok }) => (
              <option key={model.id} value={model.id}>
                {model.name} · {model.kind}
                {ok ? '' : ' (not installed)'}
              </option>
            ))}
          </select>
        </label>
        <label
          htmlFor={`${id}-prompt`}
          className="grid gap-1 text-xs font-semibold text-ink"
        >
          Prompt
          <textarea
            id={`${id}-prompt`}
            value={prompt}
            maxLength={8000}
            rows={4}
            placeholder="Describe what you want to compose…"
            onChange={(e) => setPrompt(e.target.value)}
            className="rounded-lg border border-line bg-surface-2 px-2.5 py-2 text-sm"
          />
        </label>
        <label
          htmlFor={`${id}-seed`}
          className="grid gap-1 text-xs font-semibold text-ink"
        >
          Seed
          <Input
            id={`${id}-seed`}
            type="number"
            min={0}
            max={2147483647}
            value={seed}
            onChange={(e) => setSeed(e.target.value)}
            className="min-h-11 w-40"
          />
        </label>
        {!installed && modelId && (
          <p className="m-0 text-xs text-ink-soft">
            Install this model in the Models tab first.
          </p>
        )}
        <WButton
          type="submit"
          variant="primary"
          disabled={!!lab.busy || !prompt.trim() || !installed}
        >
          <Play aria-hidden="true" size={14} />
          Generate locally
        </WButton>
      </form>
    </Panel>
  );
}

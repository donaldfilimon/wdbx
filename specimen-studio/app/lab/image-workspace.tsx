import { useEffect, useId, useRef, useState } from 'react';
import { ImagePlus, ScanText } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { CodeBlock, Panel, WButton } from '@/components/wdbx';
import {
  analyzeImage,
  learnText,
  learnVisual,
  runText,
  runVisual,
  type Analyzed,
  type Focus,
} from '@/lib/specimen/lab-api';
import type { Specimen } from '@/lib/specimen/types';
import type { Lab } from './use-lab';

const MAX_BYTES = 32 * 1024 * 1024;
const WHOLE: Focus = { x: 0, y: 0, width: 1, height: 1 };

async function readImage(
  file: File,
): Promise<{ bytes: number[]; name: string } | { error: string }> {
  if (file.size > MAX_BYTES) return { error: 'Choose an image under 32 MB.' };
  return {
    bytes: Array.from(new Uint8Array(await file.arrayBuffer())),
    name: file.name.replace(/\.[^.]+$/, ''),
  };
}

/** Image workspace: choose an image, set a focus, analyze, then learn or run. */
export function ImageWorkspace({
  lab,
  specimen,
  inbound,
}: {
  lab: Lab;
  specimen: Specimen;
  /** An image handed over from another tab (a generated artifact). */
  inbound: File | null;
}) {
  const id = useId();
  const fileInput = useRef<HTMLInputElement>(null);
  const bytes = useRef<number[]>([]);
  const [image, setImage] = useState('');
  const [aspect, setAspect] = useState(4 / 3);
  const [focus, setFocus] = useState<Focus>(WHOLE);
  const [analysis, setAnalysis] = useState<Analyzed | null>(null);
  const [ocr, setOcr] = useState('');
  const [name, setName] = useState('');
  const [chooseError, setChooseError] = useState('');
  const busy = !!lab.busy;

  useEffect(() => () => URL.revokeObjectURL(image), [image]);

  const apply = (
    file: File,
    read: { bytes: number[]; name: string } | { error: string },
  ) => {
    if ('error' in read) {
      setChooseError(read.error);
      return;
    }
    setChooseError('');
    bytes.current = read.bytes;
    setImage(URL.createObjectURL(file));
    setName(read.name);
    setAnalysis(null);
    setFocus(WHOLE);
  };
  const choose = async (file?: File) => {
    if (file) apply(file, await readImage(file));
  };
  // An image handed over from the Artifacts tab: state updates happen in the
  // read's callback, after the file is read.
  useEffect(() => {
    if (!inbound) return;
    let live = true;
    void readImage(inbound).then((read) => {
      if (live) apply(inbound, read);
    });
    return () => {
      live = false;
    };
    // Only a new handed-over file should load; `apply` is recreated each render.
    // oxlint-disable-next-line react-hooks/exhaustive-deps
  }, [inbound]);

  const analyze = () =>
    lab.work('Analyzing image', async (jobId) => {
      const result = await analyzeImage(
        bytes.current,
        focus,
        jobId,
        lab.progress,
      );
      setAnalysis(result);
      setOcr(result.analysis.ocr.map((line) => line.text).join('\n'));
    });

  const a = analysis?.analysis;
  return (
    <Panel
      title="Image workspace"
      description="Drop, paste or open an image, move the focus, then analyze its features and text."
      actions={
        <WButton
          variant="outline"
          disabled={busy}
          onClick={() => fileInput.current?.click()}
        >
          <ImagePlus aria-hidden="true" size={15} />
          Open image
        </WButton>
      }
    >
      <div className="grid gap-4 p-4">
        <input
          ref={fileInput}
          type="file"
          accept="image/png,image/jpeg,image/webp"
          hidden
          onChange={(e) => void choose(e.target.files?.[0])}
        />
        {chooseError && (
          <p role="alert" className="m-0 text-sm text-danger">
            {chooseError}
          </p>
        )}
        <button
          type="button"
          aria-label={
            image
              ? 'Image focus: click to move the focus box to that point'
              : 'Drop or paste an image here, or activate to open one'
          }
          className="relative grid w-full place-items-center overflow-hidden rounded-lg border border-dashed border-line-strong bg-surface-2 focus-visible:outline-2 focus-visible:outline-focus"
          style={{ aspectRatio: aspect }}
          onDragOver={(e) => e.preventDefault()}
          onDrop={(e) => {
            e.preventDefault();
            void choose(e.dataTransfer.files[0]);
          }}
          onPaste={(e) => {
            const f = Array.from(e.clipboardData.files)[0];
            if (f) {
              e.preventDefault();
              void choose(f);
            }
          }}
          onClick={(e) => {
            if (!image) return fileInput.current?.click();
            if (busy) return;
            const r = e.currentTarget.getBoundingClientRect();
            setFocus((f) => ({
              ...f,
              x: Math.max(
                0,
                Math.min(
                  1 - f.width,
                  (e.clientX - r.left) / r.width - f.width / 2,
                ),
              ),
              y: Math.max(
                0,
                Math.min(
                  1 - f.height,
                  (e.clientY - r.top) / r.height - f.height / 2,
                ),
              ),
            }));
            setAnalysis(null);
          }}
        >
          {image ? (
            <>
              {/* oxlint-disable-next-line next/no-img-element -- Object URLs of local files; next/image cannot optimize them. */}
              <img
                src={image}
                alt="Full frame, blurred outside the focus"
                className="absolute inset-0 size-full object-contain"
                style={{ filter: focus.width < 1 ? 'blur(3px)' : 'none' }}
                onLoad={(e) =>
                  setAspect(
                    e.currentTarget.naturalWidth /
                      e.currentTarget.naturalHeight,
                  )
                }
              />
              {/* oxlint-disable-next-line next/no-img-element -- Object URLs of local files; next/image cannot optimize them. */}
              <img
                src={image}
                alt="Focused region"
                className="absolute inset-0 size-full object-contain"
                style={{
                  clipPath: `inset(${focus.y * 100}% ${(1 - focus.x - focus.width) * 100}% ${(1 - focus.y - focus.height) * 100}% ${focus.x * 100}%)`,
                }}
              />
              <span
                aria-hidden="true"
                className="absolute border-2 border-teal"
                style={{
                  left: `${focus.x * 100}%`,
                  top: `${focus.y * 100}%`,
                  width: `${focus.width * 100}%`,
                  height: `${focus.height * 100}%`,
                }}
              />
              {a && (
                <svg
                  aria-hidden="true"
                  className="absolute inset-0 size-full"
                  viewBox="0 0 100 100"
                  preserveAspectRatio="none"
                >
                  {a.contours.map(([x, y], i) => (
                    <circle
                      key={i}
                      cx={(focus.x + x * focus.width) * 100}
                      cy={(focus.y + y * focus.height) * 100}
                      r=".25"
                      fill="var(--amber)"
                    />
                  ))}
                  {a.ocr.flatMap((line, li) =>
                    line.regions.map((region, ri) => (
                      <rect
                        key={`${li}-${ri}`}
                        x={
                          (focus.x + (region.cx - region.width / 2) / a.width) *
                          100
                        }
                        y={
                          (focus.y +
                            (region.cy - region.height / 2) / a.height) *
                          100
                        }
                        width={(region.width / a.width) * 100}
                        height={(region.height / a.height) * 100}
                        fill="none"
                        stroke="var(--info)"
                        strokeWidth=".25"
                      />
                    )),
                  )}
                </svg>
              )}
            </>
          ) : (
            <span className="grid justify-items-center gap-1 p-8 text-center text-sm text-ink-soft">
              <ImagePlus aria-hidden="true" size={34} />
              <strong className="text-ink">Drop or paste an image here</strong>
              PNG, JPEG or WebP, up to 32 MB
            </span>
          )}
        </button>
        <div className="flex flex-wrap items-end gap-3">
          <label
            htmlFor={`${id}-focus`}
            className="grid gap-1 text-xs font-semibold text-ink"
          >
            Focus size ({Math.round(focus.width * 100)}%)
            <input
              id={`${id}-focus`}
              type="range"
              min="20"
              max="100"
              value={focus.width * 100}
              disabled={busy}
              className="min-h-11 w-48 accent-teal"
              onChange={(e) => {
                const size = Number(e.target.value) / 100;
                setFocus((f) => ({
                  ...f,
                  width: size,
                  height: size,
                  x: Math.min(f.x, 1 - size),
                  y: Math.min(f.y, 1 - size),
                }));
                setAnalysis(null);
              }}
            />
          </label>
          <WButton
            variant="primary"
            disabled={!image || busy}
            onClick={() => void analyze()}
          >
            <ScanText aria-hidden="true" size={15} />
            Analyze image and text
          </WButton>
        </div>
        {analysis && a && (
          <div className="grid gap-3 rounded-lg border border-line bg-surface-2 p-3 text-sm">
            <code className="font-mono text-xs break-all text-teal-strong">
              {a.patternId}
            </code>
            <p className="m-0 text-ink-soft">
              {a.resolution} complexity · {a.contours.length} boundary points ·{' '}
              {a.holes} holes
            </p>
            <p className="m-0 text-ink-soft">{a.ocrStatus}</p>
            <label
              htmlFor={`${id}-ocr`}
              className="grid gap-1 text-xs font-semibold text-ink"
            >
              Recognized text (edit before learning)
              <textarea
                id={`${id}-ocr`}
                value={ocr}
                rows={3}
                onChange={(e) => setOcr(e.target.value)}
                className="rounded-lg border border-line bg-surface px-2.5 py-2 font-mono text-sm"
              />
            </label>
            <label
              htmlFor={`${id}-name`}
              className="grid gap-1 text-xs font-semibold text-ink"
            >
              Pattern name
              <Input
                id={`${id}-name`}
                value={name}
                onChange={(e) => setName(e.target.value)}
                className="min-h-11"
              />
            </label>
            <div className="flex flex-wrap gap-2">
              <WButton
                variant="primary"
                disabled={busy || !name.trim()}
                onClick={() =>
                  void lab.work('Learning image', async () =>
                    lab.applySnapshot(
                      await learnVisual(specimen, analysis.asset, a, name, ocr),
                    ),
                  )
                }
              >
                Learn this image
              </WButton>
              <WButton
                variant="outline"
                disabled={busy}
                onClick={() =>
                  void lab.work('Running image cycle', async (jobId) =>
                    lab.applySnapshot(
                      (
                        await runVisual(
                          specimen,
                          analysis.asset,
                          focus,
                          jobId,
                          lab.progress,
                        )
                      ).snapshot,
                    ),
                  )
                }
              >
                Run image cycle
              </WButton>
              <WButton
                variant="outline"
                disabled={busy || !ocr.trim()}
                onClick={() =>
                  void lab.work('Running recognized text', async (jobId) =>
                    lab.applySnapshot(
                      (await runText(specimen, ocr, jobId, lab.progress))
                        .snapshot,
                    ),
                  )
                }
              >
                Run recognized text
              </WButton>
              <WButton
                variant="outline"
                disabled={busy || !ocr.trim() || !name.trim()}
                onClick={() =>
                  void lab.work('Learning reviewed text', async () =>
                    lab.applySnapshot(
                      await learnText(specimen, {
                        asset: analysis.asset,
                        pattern: name,
                        text: ocr,
                      }),
                    ),
                  )
                }
              >
                Learn reviewed text
              </WButton>
            </div>
            {analysis.matches.length > 0 && (
              <ul
                aria-label="Closest learned images"
                className="m-0 grid gap-1 pl-5"
              >
                {analysis.matches.map((m, i) => (
                  <li key={i}>
                    {m.name} · {m.score.confidence.toFixed(1)} confidence
                  </li>
                ))}
              </ul>
            )}
            <details>
              <summary className="min-h-11 cursor-pointer content-center font-semibold text-ink-soft">
                Feature and signed-distance evidence
              </summary>
              <CodeBlock label="Feature and signed-distance evidence">
                {JSON.stringify(
                  {
                    features: a.features,
                    sdfRange: a.sdf.values.length
                      ? [Math.min(...a.sdf.values), Math.max(...a.sdf.values)]
                      : [],
                    convex: a.convex,
                    concave: a.concave,
                  },
                  null,
                  2,
                )}
              </CodeBlock>
            </details>
          </div>
        )}
      </div>
    </Panel>
  );
}

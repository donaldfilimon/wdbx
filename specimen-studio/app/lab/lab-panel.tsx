import { useEffect, useState } from 'react';
import { Brain, Check, Square } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { CodeBlock, EmptyState, Panel, WButton } from '@/components/wdbx';
import { assetBytes, learnText, type Artifact } from '@/lib/specimen/lab-api';
import { cancelNative } from '@/lib/specimen/native';
import type { Specimen } from '@/lib/specimen/types';
import { ImageWorkspace } from './image-workspace';
import { GeneratePanel, ModelsPanel } from './models-panel';
import { useLab, type Lab } from './use-lab';

const trigger = 'min-h-11 px-4 text-ink-soft data-active:text-ink';

/** The browser edition explains what the desktop adds. */
export function BrowserLabIntro() {
  return (
    <section className="native-lab grid gap-3">
      <Panel
        title="Bring your specimen to the desktop"
        description="The desktop edition adds image analysis, local OCR, a sparse composition network and local text and image models."
      >
        <div className="grid gap-3 p-4 text-sm text-ink-soft">
          <Brain aria-hidden="true" size={28} className="text-teal" />
          <p className="m-0">
            Your browser specimen keeps running here with browser storage. Use
            Save specimen to carry your patterns to the desktop app.
          </p>
          <p className="runtime-pills m-0 flex flex-wrap gap-2">
            <Badge variant="outline">Browser runtime</Badge>
            <Badge variant="outline">Local browser storage</Badge>
            <Badge variant="outline">Text patterns</Badge>
          </p>
        </div>
      </Panel>
    </section>
  );
}

function AssetImage({ asset }: { asset: string }) {
  const [url, setUrl] = useState('');
  useEffect(() => {
    let live = true;
    let created = '';
    assetBytes(asset).then(
      (data) => {
        created = URL.createObjectURL(
          new Blob([new Uint8Array(data)], { type: 'image/png' }),
        );
        if (live) setUrl(created);
        else URL.revokeObjectURL(created);
      },
      () => live && setUrl(''),
    );
    return () => {
      live = false;
      URL.revokeObjectURL(created);
    };
  }, [asset]);
  return url ? (
    // oxlint-disable-next-line next/no-img-element -- An object URL of a local asset; next/image cannot optimize it.
    <img
      className="max-h-64 w-full rounded object-contain"
      src={url}
      alt="Locally generated artifact"
    />
  ) : null;
}

function ArtifactsPanel({
  lab,
  specimen,
  onInspect,
}: {
  lab: Lab;
  specimen: Specimen;
  onInspect: (file: File) => void;
}) {
  const shown: Artifact[] = lab.artifacts.slice().reverse().slice(0, 100);
  if (!shown.length)
    return (
      <EmptyState
        title="No artifacts yet"
        text="Generated work appears here with its model, seed and provenance."
      />
    );
  return (
    <ul className="m-0 grid list-none gap-3 p-0 sm:grid-cols-2">
      {shown.map((a) => (
        <li
          key={a.id}
          className="grid content-start gap-2 rounded-lg border border-line bg-surface p-3 text-sm"
        >
          <strong className="break-words text-ink">{a.prompt}</strong>
          {a.text ? (
            <p className="m-0 whitespace-pre-wrap text-ink-soft">{a.text}</p>
          ) : a.asset ? (
            <AssetImage asset={a.asset} />
          ) : null}
          <p className="m-0 text-xs text-ink-soft">
            {a.model} · seed {a.seed}
          </p>
          <details className="text-xs">
            <summary className="min-h-11 cursor-pointer content-center font-semibold text-ink-soft">
              Provenance
            </summary>
            <CodeBlock label={`Provenance of ${a.id}`}>
              {JSON.stringify(a, null, 2)}
            </CodeBlock>
          </details>
          {a.text && (
            <WButton
              variant="outline"
              disabled={!!lab.busy}
              onClick={() =>
                void lab.work('Learning generated text', async () =>
                  lab.applySnapshot(
                    await learnText(specimen, { artifactId: a.id }),
                  ),
                )
              }
            >
              Learn as a text pattern
            </WButton>
          )}
          {a.asset && (
            <WButton
              variant="outline"
              disabled={!!lab.busy}
              onClick={() =>
                void lab.work('Opening generated image', async () => {
                  const data = await assetBytes(a.asset!);
                  onInspect(
                    new File([new Uint8Array(data)], `${a.id}.png`, {
                      type: 'image/png',
                    }),
                  );
                })
              }
            >
              Inspect and learn image
            </WButton>
          )}
        </li>
      ))}
    </ul>
  );
}

/** The desktop Lab: local models, image analysis and generation. */
export function LabPanel({
  specimen,
  onSnapshot,
}: {
  specimen: Specimen;
  onSnapshot: (s: Specimen) => void;
}) {
  const lab = useLab(onSnapshot);
  const [tab, setTab] = useState('image');
  const [inbound, setInbound] = useState<File | null>(null);
  const others = lab.jobs.filter((id) => id !== lab.currentJob());
  return (
    <div className="native-lab grid gap-4">
      <p className="runtime-pills m-0 flex flex-wrap gap-2">
        <Badge variant="outline">
          <Check aria-hidden="true" /> Native Rust engine
        </Badge>
        <Badge variant="outline">
          {lab.caps?.storage ?? 'WDBX v2'} transactional storage
        </Badge>
        <Badge variant="outline">Local processing</Badge>
        {lab.caps?.gpu && <Badge variant="outline">GPU: {lab.caps.gpu}</Badge>}
      </p>
      <div className="flex flex-wrap items-center gap-3 text-sm">
        <output className="text-ink-soft">
          {lab.status || 'Ready to inspect or compose'}
        </output>
        {lab.busy && (
          <WButton
            variant="outline"
            onClick={() => void cancelNative(lab.currentJob())}
          >
            <Square aria-hidden="true" size={13} />
            Cancel
          </WButton>
        )}
      </div>
      {others.map((id) => (
        <div key={id} className="flex items-center gap-3 text-sm text-ink-soft">
          Another desktop job is running.
          <WButton variant="outline" onClick={() => void cancelNative(id)}>
            Cancel that job
          </WButton>
        </div>
      ))}
      {lab.error && (
        <p role="alert" className="m-0 text-sm text-danger">
          {lab.error}
        </p>
      )}
      {lab.pollError && (
        <p className="m-0 text-xs text-ink-soft">{lab.pollError}</p>
      )}
      <Tabs
        value={tab}
        onValueChange={(v) => setTab(String(v))}
        className="flex-col"
      >
        <TabsList className="h-auto flex-wrap">
          <TabsTrigger value="image" className={trigger}>
            Image
          </TabsTrigger>
          <TabsTrigger value="generate" className={trigger}>
            Generate
          </TabsTrigger>
          <TabsTrigger value="models" className={trigger}>
            Models
          </TabsTrigger>
          <TabsTrigger value="artifacts" className={trigger}>
            Artifacts ({lab.artifacts.length})
          </TabsTrigger>
        </TabsList>
        <TabsContent value="image" keepMounted>
          <ImageWorkspace lab={lab} specimen={specimen} inbound={inbound} />
        </TabsContent>
        <TabsContent value="generate" keepMounted>
          <GeneratePanel lab={lab} specimen={specimen} />
        </TabsContent>
        <TabsContent value="models">
          <ModelsPanel lab={lab} />
        </TabsContent>
        <TabsContent value="artifacts" keepMounted>
          <ArtifactsPanel
            lab={lab}
            specimen={specimen}
            onInspect={(file) => {
              setInbound(file);
              setTab('image');
            }}
          />
        </TabsContent>
      </Tabs>
    </div>
  );
}

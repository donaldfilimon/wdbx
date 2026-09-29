/**
 * Typed client for the desktop Lab: local models, image analysis and OCR,
 * generation and artifacts. Every call goes through the protocol-gated
 * `native_call` (see `native/specimen-core/src/protocol.rs`); results that
 * reach the UI are parsed here rather than cast.
 */
import {
  acceptSnapshot,
  callNative,
  nativeOperation,
  persistNative,
  type NativeSnapshot,
} from './native';
import type { Specimen } from './types';

export type Progress = (event: Record<string, unknown>) => void;

export interface LabModel {
  id: string;
  name: string;
  kind: string;
  size?: number;
  source?: string;
  license?: string;
  revision?: string;
  sha256?: string;
  memoryBytes?: number;
}
export interface ModelStatus {
  model: LabModel;
  installed: boolean;
  loaded: boolean;
}
export interface Capabilities {
  runtime: string;
  storage: string;
  ocr: boolean;
  textGeneration: boolean;
  imageGeneration: boolean;
  gpu: string;
  nativeVersion: string;
}
export interface Focus {
  x: number;
  y: number;
  width: number;
  height: number;
}
export interface OcrLine {
  text: string;
  regions: { cx: number; cy: number; width: number; height: number }[];
}
export interface Analysis {
  width: number;
  height: number;
  patternId: string;
  resolution: string;
  contours: [number, number][];
  holes: number;
  convex: number;
  concave: number;
  ocr: OcrLine[];
  ocrStatus: string;
  features: number[];
  sdf: { values: number[] };
  focus: Focus;
}
export interface Analyzed {
  analysis: Analysis;
  asset: string;
  matches: { name: string; score: { confidence: number } }[];
}
export interface Artifact {
  id: string;
  model: string;
  prompt: string;
  seed: number;
  modelDigest: string;
  createdAt: string;
  text?: string;
  asset?: string;
}

export const isBusy = (value: unknown): value is { busy: true } =>
  !!value &&
  typeof value === 'object' &&
  (value as { busy?: unknown }).busy === true;

/** Model catalog with install/load state; null while the store is busy. */
export async function modelStatus(): Promise<ModelStatus[] | null> {
  const status = await callNative<unknown>({ op: 'models' });
  return isBusy(status) || !Array.isArray(status)
    ? null
    : (status as ModelStatus[]);
}
export const capabilities = () =>
  callNative<Capabilities>({ op: 'capabilities' });
export const activeJobs = () => callNative<string[]>({ op: 'jobs' });
export const loadSnapshot = async () =>
  acceptSnapshot(await callNative<NativeSnapshot>({ op: 'snapshot' }));

const str = (v: unknown): v is string => typeof v === 'string';

/** The snapshot's artifacts, keeping only well-formed records. */
export function artifactsOf(
  snapshot: Pick<NativeSnapshot, 'artifacts'>,
): Artifact[] {
  return (snapshot.artifacts ?? []).filter(
    (a): a is Artifact & Record<string, unknown> => {
      const r = a as Record<string, unknown> | null;
      return (
        !!r &&
        str(r.id) &&
        str(r.model) &&
        str(r.prompt) &&
        typeof r.seed === 'number' &&
        (r.text === undefined || str(r.text)) &&
        (r.asset === undefined || str(r.asset))
      );
    },
  ) as Artifact[];
}

export const installModel = (
  modelId: string,
  jobId: string,
  progress?: Progress,
  path?: string,
) => nativeOperation({ op: 'installModel', modelId, path, jobId }, progress);
export const loadModel = (
  modelId: string,
  jobId: string,
  progress?: Progress,
) => nativeOperation({ op: 'loadModel', modelId, jobId }, progress);
export const unloadModel = () => nativeOperation({ op: 'unloadModel' });
export const removeModel = (modelId: string) =>
  nativeOperation({ op: 'removeModel', modelId });

export const analyzeImage = (
  bytes: number[],
  focus: Focus,
  jobId: string,
  progress?: Progress,
) =>
  nativeOperation<Analyzed>({ op: 'analyze', bytes, focus, jobId }, progress);
export const assetBytes = (id: string) =>
  callNative<number[]>({ op: 'asset', id });

/** Writes that change the specimen first persist the current edit. */
export async function learnVisual(
  specimen: Specimen,
  asset: string,
  analysis: Analysis,
  name: string,
  ocrCorrection: string,
) {
  await persistNative(specimen);
  return nativeOperation<NativeSnapshot>({
    op: 'learnVisual',
    asset,
    analysis,
    name,
    ocrCorrection,
  });
}
export async function runVisual(
  specimen: Specimen,
  asset: string,
  focus: Focus,
  jobId: string,
  progress?: Progress,
) {
  await persistNative(specimen);
  return nativeOperation<{ snapshot: NativeSnapshot }>(
    { op: 'runVisual', asset, focus, jobId },
    progress,
  );
}
export async function runText(
  specimen: Specimen,
  input: string,
  jobId: string,
  progress?: Progress,
) {
  await persistNative(specimen);
  return nativeOperation<{ snapshot: NativeSnapshot }>(
    { op: 'run', input, jobId },
    progress,
  );
}
export async function learnText(
  specimen: Specimen,
  source:
    | { artifactId: string }
    | { asset: string; pattern: string; text: string },
) {
  await persistNative(specimen);
  return nativeOperation<NativeSnapshot>({ op: 'learnText', ...source });
}
export const generate = (
  modelId: string,
  prompt: string,
  seed: number,
  jobId: string,
  progress?: Progress,
) =>
  nativeOperation<{ snapshot: NativeSnapshot; artifact: Artifact }>(
    { op: 'generate', modelId, prompt, seed, jobId },
    progress,
  );

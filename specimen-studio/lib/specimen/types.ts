export type Resolution = 'low' | 'medium' | 'high';
export type NodeType = 'pattern' | 'A' | 'B';
export type Tone = 'neutral' | 'warm' | 'curious' | 'cautious';
export interface Alternative {
  id: string;
  action: string;
  inhibition: string;
  weight: number;
  remixed: boolean;
}
export interface Entry {
  id: string;
  pattern: string;
  alternatives: Alternative[];
}
export interface SpecimenNode {
  ref: string;
  name: string;
  patternId: string;
  resolution: Resolution;
  type: NodeType;
  contextId: string;
  strength: number;
  jitter: boolean;
  tone: Tone;
  entries: Entry[];
  createdAt: string;
}
export interface Attachment {
  id: string;
  from: string;
  to: string;
  bidirectional: boolean;
  hard: boolean;
  affinity: number;
}
export interface Resource {
  ref: string;
  subsystem: string;
  text: string;
  value: string;
  patternId: string;
  resourceId: string;
  valence: number;
  intensity: number;
}
export interface TraceStep {
  id: string;
  phase: string;
  detail: string;
  count?: number;
  duration?: number;
}
export interface Vote {
  id: string;
  nodeRef: string;
  entryRef: string;
  action: string;
  base: number;
  confidence: number;
  strength: number;
  input: string;
  group?: string;
  resources: string[];
  evidence?: {
    similarity: number;
    dissimilarity: number;
    modulation: number;
    jitter: number;
    confidence: number;
  };
  origin?: number;
  binding?: unknown;
}
export interface Segment {
  id: string;
  text: string;
  contributors: string[];
  sourceVotes: string[];
  transformations: string[];
  color: string;
}
export interface Cycle {
  id: string;
  input: string;
  createdAt: string;
  segments: Segment[];
  votes: Vote[];
  trace: TraceStep[];
  duration: number;
  feedback: string[];
  status: 'complete' | 'unmatched' | 'inhibited' | 'cancelled';
  pinned: boolean;
  visual?: Visual;
  supervisedUntil: number;
  reviewed?: boolean;
}
export interface Visual {
  synthesis?: {
    backend: string;
    activationFunctions: string[];
    fallback?: string;
    networkVersion: number;
    seed: number;
    brainstorm: boolean;
  };
  xArray: number[];
  yArray: number[];
  colorArray: string[];
  brightnessArray: number[];
  size: { width: number; height: number };
  position: { x: number; y: number };
}
export interface Mutation {
  id: string;
  recipient: string;
  donor: string;
  slot: string;
  originalPattern: string;
  originalAction: string;
  createdAt: string;
}
export interface EventRecord {
  id: string;
  type: string;
  title: string;
  detail: string;
  createdAt: string;
}
export interface Proposal {
  id: string;
  pattern: string;
  evidence: number;
  status: 'pending' | 'learned' | 'dismissed';
}
export interface Settings {
  gpu?: boolean;
  voteThreshold: number;
  jitter: number;
  maxStrength: number;
  initialStrength: number;
  entryLimit: number;
  scanLimit: number;
  fanoutLimit: number;
  historyLimit: number;
  pinLimit: number;
  maxNodes: number;
  maxRafts: number;
  maxThreads: number;
  chunkSize: number;
  raftThreshold: number;
  brainstorm: boolean;
  maintenance: boolean;
  adaptiveThreading: boolean;
  idleMin: number;
  idleMax: number;
  transientWidth: number;
  seed: number;
}
export interface Specimen {
  schema: 'wdbx.studio.v1';
  name: string;
  nodes: SpecimenNode[];
  resources: Resource[];
  attachments: Attachment[];
  history: Cycle[];
  events: EventRecord[];
  mutations: Mutation[];
  proposals: Proposal[];
  atp: { valence: number; intensity: number; lastUpdate: number };
  settings: Settings;
  updatedAt: string;
}
export const SUBSYSTEMS = [
  'dictionary',
  'thesaurus',
  'antiThesaurus',
  'chargebook',
  'verbSemantics',
  'conjunctiveRules',
  'negation',
  'morphology',
  'roleMapper',
  'sigils',
  'inhibition',
] as const;
export const COLORS = ['green', 'violet', 'amber', 'blue', 'rose', 'cyan'];
export const uid = () => {
  const webCrypto =
    typeof globalThis.crypto === 'undefined' ? undefined : globalThis.crypto;
  if (typeof webCrypto?.randomUUID === 'function')
    return webCrypto.randomUUID();
  const bytes = new Uint8Array(16);
  if (typeof webCrypto?.getRandomValues === 'function')
    webCrypto.getRandomValues(bytes);
  else
    for (let index = 0; index < bytes.length; index++)
      bytes[index] = Math.floor(Math.random() * 256);
  bytes[6] = (bytes[6] & 0x0f) | 0x40;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  const value = [...bytes].map((byte) => byte.toString(16).padStart(2, '0'));
  return `${value.slice(0, 4).join('')}-${value.slice(4, 6).join('')}-${value.slice(6, 8).join('')}-${value.slice(8, 10).join('')}-${value.slice(10).join('')}`;
};
export const clamp = (n: number, min: number, max: number) =>
  Math.max(min, Math.min(max, n));

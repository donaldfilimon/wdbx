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
  evidence?: { similarity:number; dissimilarity:number; modulation:number; jitter:number; confidence:number };
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
export const uid = () => crypto.randomUUID();
export const clamp = (n: number, min: number, max: number) =>
  Math.max(min, Math.min(max, n));

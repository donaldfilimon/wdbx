import {
  Activity,
  BookOpen,
  Database,
  FileText,
  Gauge,
  HardDrive,
  Waypoints,
  Network,
  Sparkles,
} from 'lucide-react';

import type { View } from '../state/navigation';

export const views: { id: View; label: string; icon: typeof Network }[] = [
  { id: 'studio', label: 'Studio', icon: Network },
  { id: 'nodes', label: 'Node Library', icon: BookOpen },
  { id: 'memory', label: 'Memory', icon: Database },
  { id: 'network', label: 'Neural layers', icon: Waypoints },
  { id: 'activity', label: 'Activity', icon: Activity },
  { id: 'engine', label: 'Live engine', icon: Gauge },
  { id: 'store', label: 'Store', icon: HardDrive },
  { id: 'lab', label: 'Vision & models', icon: Sparkles },
  { id: 'specification', label: 'Specification', icon: FileText },
];

export const viewGroups: { label: string; ids: View[] }[] = [
  { label: 'Build', ids: ['studio', 'nodes', 'memory', 'network'] },
  { label: 'Observe', ids: ['activity', 'engine', 'store', 'lab'] },
  { label: 'Reference', ids: ['specification'] },
];

export const labels: Record<View, [string, string]> = {
  studio: ['Specimen studio', 'Teach a pattern. Trace a response.'],
  lab: [
    'Vision & local models',
    'Inspect an image. Compose locally. Learn deliberately.',
  ],
  nodes: ['Node library', 'Small patterns, reusable actions.'],
  memory: ['Memory', 'Supporting knowledge, close at hand.'],
  network: [
    'Neural layers',
    'Build the composition network. See the specimen in 3D.',
  ],
  activity: ['Activity', 'Follow the changes that shape your specimen.'],
  engine: [
    'Live engine',
    'Watch the kernel work: trace, votes, history and ATP.',
  ],
  store: [
    'Store explorer',
    'What this edition keeps on disk, and how it verifies it.',
  ],
  specification: [
    'Specification',
    'The complete architecture, alongside your specimen.',
  ],
  settings: ['Settings', 'Set the boundaries for your specimen.'],
};

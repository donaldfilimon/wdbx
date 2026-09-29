import {
  Activity,
  BookOpen,
  Database,
  FileText,
  Network,
  Sparkles,
} from 'lucide-react';

import type { View } from '../state/navigation';

export const views: { id: View; label: string; icon: typeof Network }[] = [
  { id: 'studio', label: 'Studio', icon: Network },
  { id: 'nodes', label: 'Node Library', icon: BookOpen },
  { id: 'memory', label: 'Memory', icon: Database },
  { id: 'activity', label: 'Activity', icon: Activity },
  { id: 'lab', label: 'Vision & models', icon: Sparkles },
  { id: 'specification', label: 'Specification', icon: FileText },
];

export const viewGroups: { label: string; ids: View[] }[] = [
  { label: 'Build', ids: ['studio', 'nodes', 'memory'] },
  { label: 'Observe', ids: ['activity', 'lab'] },
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
  activity: ['Activity', 'Follow the changes that shape your specimen.'],
  specification: [
    'Specification',
    'The complete architecture, alongside your specimen.',
  ],
  settings: ['Settings', 'Set the boundaries for your specimen.'],
};

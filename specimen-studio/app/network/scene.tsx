/**
 * The Network view's 3D scenes (lazy-loaded with three and
 * @react-three/fiber). Rendering is on demand; nothing animates on its own,
 * so reduced motion needs no special case. Colours come from theme tokens.
 */
import { useLayoutEffect, useMemo, useRef } from 'react';
import { Canvas, useThree, type ThreeEvent } from '@react-three/fiber';
import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import type { Network } from '@/lib/specimen/kernel';
import {
  neuronPositions,
  sampledEdges,
  topologyLayout,
} from '@/lib/specimen/network-view';
import type { Attachment, SpecimenNode } from '@/lib/specimen/types';

const CAP = 256;
const EDGES_PER_LAYER = 600;

function token(name: string, fallback: string) {
  if (typeof document === 'undefined') return fallback;
  return (
    getComputedStyle(document.documentElement).getPropertyValue(name).trim() ||
    fallback
  );
}

/** Orbit controls that request a frame only when the camera moves. */
function Controls() {
  const { camera, gl, invalidate } = useThree();
  useLayoutEffect(() => {
    const controls = new OrbitControls(camera, gl.domElement);
    controls.enableDamping = false;
    // Passing `invalidate` directly would hand it the event as a frame count.
    const redraw = () => invalidate();
    controls.addEventListener('change', redraw);
    return () => {
      controls.removeEventListener('change', redraw);
      controls.dispose();
    };
  }, [camera, gl, invalidate]);
  return null;
}

function Neurons({
  points,
  values,
}: {
  points: [number, number, number][];
  values?: number[];
}) {
  const ref = useRef<THREE.InstancedMesh>(null);
  const { invalidate } = useThree();
  useLayoutEffect(() => {
    const mesh = ref.current;
    if (!mesh) return;
    const matrix = new THREE.Matrix4();
    const idle = new THREE.Color(token('--line-strong', '#888'));
    const hot = new THREE.Color(token('--teal', '#2ec4b6'));
    const max = values?.length ? Math.max(...values, 1e-6) : 1;
    points.forEach((p, i) => {
      matrix.setPosition(p[0], p[1], p[2]);
      mesh.setMatrixAt(i, matrix);
      const v = values?.[i];
      mesh.setColorAt(
        i,
        v === undefined ? idle : idle.clone().lerp(hot, Math.max(0, v) / max),
      );
    });
    mesh.instanceMatrix.needsUpdate = true;
    if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
    invalidate();
  }, [points, values, invalidate]);
  return (
    <instancedMesh ref={ref} args={[undefined, undefined, points.length]}>
      <sphereGeometry args={[0.11, 10, 10]} />
      <meshBasicMaterial />
    </instancedMesh>
  );
}

function Segments({
  pairs,
  opacity,
}: {
  pairs: [[number, number, number], [number, number, number]][];
  opacity: number;
}) {
  const geometry = useMemo(() => {
    const g = new THREE.BufferGeometry();
    g.setAttribute(
      'position',
      new THREE.Float32BufferAttribute(pairs.flat(2), 3),
    );
    return g;
  }, [pairs]);
  useLayoutEffect(() => () => geometry.dispose(), [geometry]);
  return (
    <lineSegments geometry={geometry}>
      <lineBasicMaterial
        color={token('--ink-soft', '#999')}
        transparent
        opacity={opacity}
      />
    </lineSegments>
  );
}

/** Layer columns with sampled sparse connections, lit by the last trace. */
export function LayersScene({
  network,
  trace,
}: {
  network: Network;
  trace?: number[][];
}) {
  const columns = useMemo(() => neuronPositions(network, CAP), [network]);
  const edges = useMemo(
    () =>
      network.layers.flatMap((layer, i) =>
        sampledEdges(layer, CAP, EDGES_PER_LAYER).map(
          ([from, to]) =>
            [columns[i][from], columns[i + 1][to]] as [
              [number, number, number],
              [number, number, number],
            ],
        ),
      ),
    [network, columns],
  );
  return (
    <Canvas frameloop="demand" camera={{ position: [0, 3, 10], fov: 45 }}>
      <Controls />
      {columns.map((points, i) => (
        <Neurons
          key={`${i}-${points.length}`}
          points={points}
          values={i === 0 ? undefined : trace?.[i - 1]?.slice(0, CAP)}
        />
      ))}
      <Segments pairs={edges} opacity={0.12} />
    </Canvas>
  );
}

const TONE: Record<SpecimenNode['tone'], [string, string]> = {
  neutral: ['--ink-soft', '#999'],
  warm: ['--amber', '#e0a040'],
  curious: ['--violet', '#9b7bd8'],
  cautious: ['--info', '#4a9bd0'],
};

function NodeMesh({
  node,
  position,
  maxStrength,
  selected,
  onSelect,
}: {
  node: SpecimenNode;
  position: [number, number, number];
  maxStrength: number;
  selected: boolean;
  onSelect: (ref: string) => void;
}) {
  const size =
    (0.18 + (0.3 * node.strength) / maxStrength) * (selected ? 1.4 : 1);
  const color = selected
    ? token('--teal', '#2ec4b6')
    : token(...TONE[node.tone]);
  return (
    <mesh
      position={position}
      onClick={(e: ThreeEvent<MouseEvent>) => {
        e.stopPropagation();
        onSelect(node.ref);
      }}
    >
      {node.type === 'A' ? (
        <boxGeometry args={[size * 1.6, size * 1.6, size * 1.6]} />
      ) : node.type === 'B' ? (
        <octahedronGeometry args={[size * 1.2]} />
      ) : (
        <sphereGeometry args={[size, 16, 16]} />
      )}
      <meshBasicMaterial color={color} />
    </mesh>
  );
}

/** Specimen nodes and attachments; clicking a node selects it. */
export function TopologyScene({
  nodes,
  attachments,
  maxStrength,
  selected,
  onSelect,
}: {
  nodes: SpecimenNode[];
  attachments: Attachment[];
  maxStrength: number;
  selected?: string;
  onSelect: (ref: string) => void;
}) {
  const layout = useMemo(() => topologyLayout(nodes), [nodes]);
  const hard = attachments.filter(
    (a) => a.hard && layout[a.from] && layout[a.to],
  );
  const soft = attachments.filter(
    (a) => !a.hard && layout[a.from] && layout[a.to],
  );
  const pairs = (list: Attachment[]) =>
    list.map(
      (a) =>
        [layout[a.from], layout[a.to]] as [
          [number, number, number],
          [number, number, number],
        ],
    );
  return (
    <Canvas frameloop="demand" camera={{ position: [0, 0, 10], fov: 45 }}>
      <Controls />
      {nodes.map((node) => (
        <NodeMesh
          key={node.ref}
          node={node}
          position={layout[node.ref]}
          maxStrength={maxStrength}
          selected={node.ref === selected}
          onSelect={onSelect}
        />
      ))}
      <Segments pairs={pairs(hard)} opacity={0.9} />
      <Segments pairs={pairs(soft)} opacity={0.3} />
    </Canvas>
  );
}

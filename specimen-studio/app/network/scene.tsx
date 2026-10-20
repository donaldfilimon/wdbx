/**
 * The Network view's 3D scenes (lazy-loaded with three and
 * @react-three/fiber). Rendering is on demand; nothing animates on its own,
 * so reduced motion needs no special case. Colours come from theme tokens.
 */
import { useLayoutEffect, useMemo, useRef, useState } from 'react';
import { Canvas, useThree, type ThreeEvent } from '@react-three/fiber';
import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import type { Network } from '@/lib/specimen/kernel';
import { neuronPositions, sampledEdges } from '@/lib/specimen/network-view';
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

/** Reads the theme class so a theme change repaints token colours. */
function useThemeFrame() {
  const [frame, setFrame] = useState(0);
  useLayoutEffect(() => {
    const root = document.documentElement;
    const observer = new MutationObserver(() => setFrame((n) => n + 1));
    observer.observe(root, { attributes: true, attributeFilter: ['class'] });
    return () => observer.disconnect();
  }, []);
  return frame;
}

function ThemePaint({ frame }: { frame: number }) {
  const { invalidate } = useThree();
  useLayoutEffect(() => {
    invalidate();
  }, [frame, invalidate]);
  return null;
}

/** Orbit controls that request a frame only when the camera moves. */
function Controls() {
  const { camera, gl, invalidate } = useThree();
  useLayoutEffect(() => {
    const controls = new OrbitControls(camera, gl.domElement);
    // Reduced motion keeps the camera still: no damping and no auto-rotate.
    const reduced = window.matchMedia('(prefers-reduced-motion: reduce)');
    const holdStill = () => {
      controls.autoRotate = false;
      controls.enableDamping = false;
    };
    holdStill();
    reduced.addEventListener('change', holdStill);
    // Passing `invalidate` directly would hand it the event as a frame count.
    const redraw = () => invalidate();
    controls.addEventListener('change', redraw);
    return () => {
      reduced.removeEventListener('change', holdStill);
      controls.removeEventListener('change', redraw);
      controls.dispose();
    };
  }, [camera, gl, invalidate]);
  return null;
}

function Neurons({
  points,
  values,
  frame,
}: {
  points: [number, number, number][];
  values?: number[];
  frame: number;
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
  }, [points, values, frame, invalidate]);
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
  frame,
}: {
  pairs: [[number, number, number], [number, number, number]][];
  opacity: number;
  frame: number;
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
  const color = token('--ink-soft', '#999');
  return (
    <lineSegments geometry={geometry} key={frame}>
      <lineBasicMaterial color={color} transparent opacity={opacity} />
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
  const frame = useThemeFrame();
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
      <ThemePaint frame={frame} />
      <Controls />
      {columns.map((points, i) => (
        <Neurons
          key={`${i}-${points.length}`}
          points={points}
          values={i === 0 ? undefined : trace?.[i - 1]?.slice(0, CAP)}
          frame={frame}
        />
      ))}
      <Segments pairs={edges} opacity={0.12} frame={frame} />
    </Canvas>
  );
}

const TONE: Record<SpecimenNode['tone'], [string, string]> = {
  neutral: ['--ink-soft', '#999'],
  warm: ['--amber', '#e0a040'],
  curious: ['--violet', '#9b7bd8'],
  cautious: ['--info', '#4a9bd0'],
};

type Vec3 = [number, number, number];

function clampCoord(n: number) {
  return Math.max(-100, Math.min(100, n));
}

function NodeMesh({
  node,
  position,
  maxStrength,
  selected,
  frame,
  linking,
  onLink,
  onSelect,
  onMove,
  onConnect,
}: {
  node: SpecimenNode;
  position: Vec3;
  maxStrength: number;
  selected: boolean;
  frame: number;
  linking: string | null;
  onLink: (ref: string | null) => void;
  onSelect: (ref: string) => void;
  onMove: (ref: string, x: number, y: number, z: number) => void;
  onConnect: (from: string, to: string) => void;
}) {
  const { camera, invalidate } = useThree();
  const [live, setLive] = useState<Vec3 | null>(null);
  const drag = useRef<{
    pointer: number;
    origin: THREE.Vector3;
    point: THREE.Vector3;
    plane: THREE.Plane;
    moved: boolean;
    connect: boolean;
  } | null>(null);
  const size =
    (0.18 + (0.3 * node.strength) / maxStrength) * (selected ? 1.4 : 1);
  const color = selected
    ? token('--teal', '#2ec4b6')
    : token(...TONE[node.tone]);
  const shown = live ?? position;
  const release = (event: ThreeEvent<PointerEvent>) => {
    const current = drag.current;
    if (!current || event.pointerId !== current.pointer) return;
    drag.current = null;
    setLive(null);
    if (current.connect) return;
    if (current.moved)
      onMove(node.ref, current.point.x, current.point.y, current.point.z);
    else onSelect(node.ref);
  };
  return (
    <group position={shown}>
      <mesh
        userData={{ nodeRef: node.ref }}
        onPointerDown={(event: ThreeEvent<PointerEvent>) => {
          if (event.button !== 0) return;
          event.stopPropagation();
          const origin = new THREE.Vector3(...shown);
          const normal = new THREE.Vector3();
          camera.getWorldDirection(normal);
          drag.current = {
            pointer: event.pointerId,
            origin,
            point: origin.clone(),
            plane: new THREE.Plane().setFromNormalAndCoplanarPoint(
              normal,
              origin,
            ),
            moved: false,
            connect: false,
          };
        }}
        onPointerMove={(event: ThreeEvent<PointerEvent>) => {
          const current = drag.current;
          if (!current || current.connect || event.pointerId !== current.pointer)
            return;
          event.stopPropagation();
          const hit = new THREE.Vector3();
          if (!event.ray.intersectPlane(current.plane, hit)) return;
          hit.set(clampCoord(hit.x), clampCoord(hit.y), clampCoord(hit.z));
          if (hit.distanceTo(current.origin) > 0.05) current.moved = true;
          current.point.copy(hit);
          setLive([hit.x, hit.y, hit.z]);
          invalidate();
        }}
        onPointerUp={(event: ThreeEvent<PointerEvent>) => {
          if (linking && linking !== node.ref) {
            onLink(null);
            onConnect(linking, node.ref);
            return;
          }
          release(event);
        }}
      >
        {node.type === 'A' ? (
          <boxGeometry args={[size * 1.6, size * 1.6, size * 1.6]} />
        ) : node.type === 'B' ? (
          <octahedronGeometry args={[size * 1.2]} />
        ) : (
          <sphereGeometry args={[size, 16, 16]} />
        )}
        <meshBasicMaterial color={color} key={frame} />
      </mesh>
      <mesh
        position={[size, size, 0]}
        onPointerDown={(event: ThreeEvent<PointerEvent>) => {
          event.stopPropagation();
          onLink(node.ref);
          drag.current = {
            pointer: event.pointerId,
            origin: new THREE.Vector3(),
            point: new THREE.Vector3(),
            plane: new THREE.Plane(),
            moved: false,
            connect: true,
          };
        }}
        onPointerUp={(event: ThreeEvent<PointerEvent>) => {
          event.stopPropagation();
          if (linking === node.ref) {
            onLink(null);
            drag.current = null;
          }
        }}
      >
        <sphereGeometry args={[0.08, 8, 8]} />
        <meshBasicMaterial color={token('--teal', '#2ec4b6')} />
      </mesh>
    </group>
  );
}

/** Specimen nodes and attachments. Drag a node to pin it; drag its handle to connect. */
export function TopologyScene({
  nodes,
  attachments,
  maxStrength,
  selected,
  positions,
  onSelect,
  onMove,
  onConnect,
}: {
  nodes: SpecimenNode[];
  attachments: Attachment[];
  maxStrength: number;
  selected?: string;
  positions: Record<string, Vec3>;
  onSelect: (ref: string) => void;
  onMove: (ref: string, x: number, y: number, z: number) => void;
  onConnect: (from: string, to: string) => void;
}) {
  const frame = useThemeFrame();
  const [linking, setLinking] = useState<string | null>(null);
  const hard = attachments.filter(
    (a) => a.hard && positions[a.from] && positions[a.to],
  );
  const soft = attachments.filter(
    (a) => !a.hard && positions[a.from] && positions[a.to],
  );
  const pairs = (list: Attachment[]) =>
    list.map(
      (a) =>
        [positions[a.from], positions[a.to]] as [Vec3, Vec3],
    );
  return (
    <Canvas
      frameloop="demand"
      camera={{ position: [0, 0, 10], fov: 45 }}
      onPointerMissed={() => setLinking(null)}
    >
      <ThemePaint frame={frame} />
      <Controls />
      {nodes.map((node) => (
        <NodeMesh
          key={node.ref}
          node={node}
          position={positions[node.ref]}
          maxStrength={maxStrength}
          selected={node.ref === selected}
          frame={frame}
          linking={linking}
          onLink={setLinking}
          onSelect={onSelect}
          onMove={onMove}
          onConnect={onConnect}
        />
      ))}
      <Segments pairs={pairs(hard)} opacity={0.9} frame={frame} />
      <Segments pairs={pairs(soft)} opacity={0.3} frame={frame} />
    </Canvas>
  );
}

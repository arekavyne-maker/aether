import test from 'node:test';
import assert from 'node:assert/strict';
import { hash, forceLayout, layeredLayout, radialLayout, circularLayout, fit, diffGraph } from './layout.js';
import { treemap } from './treemap.js';

function model() {
  const nodes = [
    { id: 'a', type: 'SimulationRun' },
    { id: 'b', type: 'Result' },
    { id: 'c', type: 'Evidence' },
    { id: 'd', type: 'Requirement' },
  ];
  const byId = new Map(nodes.map((node, index) => [node.id, index]));
  const edges = [
    { id: 'e1', source: 'a', target: 'b', sourceIndex: 0, targetIndex: 1 },
    { id: 'e2', source: 'a', target: 'c', sourceIndex: 0, targetIndex: 2 },
  ];
  return { nodes, edges, byId, degree: [2, 1, 1, 0] };
}

test('hash and layouts are deterministic', () => {
  assert.equal(hash('Requirement'), hash('Requirement'));
  const m = model();
  assert.deepEqual(forceLayout(m).positions, forceLayout(m).positions);
  assert.deepEqual(layeredLayout(m).positions, layeredLayout(m).positions);
  assert.deepEqual(radialLayout(m, 'a').positions, radialLayout(m, 'a').positions);
  assert.deepEqual(circularLayout(m).positions, circularLayout(m).positions);
});

test('every node gets a finite position', () => {
  const m = model();
  for (const layout of [forceLayout(m), layeredLayout(m), radialLayout(m, 'b'), circularLayout(m)]) {
    assert.equal(Object.keys(layout.positions).length, m.nodes.length);
    for (const point of Object.values(layout.positions)) {
      assert.ok(Number.isFinite(point.x) && Number.isFinite(point.y));
    }
  }
});

test('fit keeps all points inside the viewport', () => {
  const raw = forceLayout(model());
  const mapped = fit(raw.positions, raw.bounds, 800, 600, 20);
  for (const point of Object.values(mapped.positions)) {
    assert.ok(point.x >= 0 && point.x <= 800, `x=${point.x}`);
    assert.ok(point.y >= 0 && point.y <= 600, `y=${point.y}`);
  }
});

test('diff reports exact added and removed ids', () => {
  const before = { nodes: [{ id: 'a' }, { id: 'b' }], edges: [{ id: 'e1' }] };
  const after = { nodes: [{ id: 'a' }, { id: 'c' }], edges: [{ id: 'e2' }] };
  assert.deepEqual(diffGraph(before, after), {
    addedNodes: ['c'], removedNodes: ['b'], addedEdges: ['e2'], removedEdges: ['e1'],
  });
});

test('treemap tiles are non-overlapping and cover the canvas', () => {
  const rects = treemap([
    { family: 'graph', value: 4 }, { family: 'vv', value: 3 }, { family: 'status', value: 1 },
  ], 100, 100, 0);
  assert.equal(rects.length, 3);
  const area = rects.reduce((sum, rect) => sum + rect.w * rect.h, 0);
  assert.ok(Math.abs(area - 10000) < 1, `area=${area}`);
  for (const rect of rects) {
    assert.ok(rect.x >= 0 && rect.y >= 0 && rect.x + rect.w <= 100.01 && rect.y + rect.h <= 100.01);
  }
});

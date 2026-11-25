import test from 'node:test';
import assert from 'node:assert/strict';
import { normalize, graphModel, nodeCounts, edgeCounts, coverageSummary } from './atlas.js';

const raw = {
  project: { root: '/tmp/project', initialized: true },
  chain: { valid: true, event_count: 5 },
  status: { version: '3.0.0-rc.1', contracts: 32 },
  graph: {
    offset: 5, event_count: 5, node_count: 2, edge_count: 1,
    nodes_returned: 2, edges_returned: 1, truncated: false,
    nodes: [
      { id: 'n1', type_id: 'Requirement', state: 'draft', properties: { text: 'x' } },
      { id: 'n2', type_id: 'Evidence', state: 'released', properties: {} },
    ],
    edges: [{ id: 'e1', type_id: 'verifies', source: 'n2', target: 'n1' }],
  },
  schema: { node_types: ['Requirement', 'Evidence'], edge_types: ['verifies'], contract_count: 32 },
  coverage: { requirements_total: 2, requirements_verified: 1, gaps: [{ requirement_id: 'n1', reason_code: 'NO_VERIFICATION_CASE' }] },
  sources: [{ command: 'aether status', ok: true, duration_ms: 3 }],
};

test('normalize preserves CLI counts and sources', () => {
  const atlas = normalize(raw);
  assert.equal(atlas.graph.offset, 5);
  assert.equal(atlas.graph.nodeCount, 2);
  assert.equal(atlas.graph.edgeCount, 1);
  assert.equal(atlas.sources.length, 1);
  assert.equal(atlas.timing.durationMs, 3);
});

test('graph model drops dangling edges and computes degree', () => {
  const model = graphModel(raw.graph);
  assert.equal(model.nodes.length, 2);
  assert.equal(model.edges.length, 1);
  assert.equal(model.degree[0], 1);
  assert.equal(model.degree[1], 1);
  assert.deepEqual(nodeCounts(raw.graph.nodes), [['Evidence', 1], ['Requirement', 1]]);
  assert.deepEqual(edgeCounts(raw.graph.edges), [['verifies', 1]]);
});

test('coverage summary does not invent values', () => {
  const summary = coverageSummary(raw.coverage);
  assert.equal(summary.percent, 50);
  assert.equal(summary.gaps, 1);
  assert.equal(coverageSummary(null), null);
});

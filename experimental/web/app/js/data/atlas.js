// Project atlas data access. Every field originates in the canonical CLI.
import { api } from '../api.js';

/** Load the full project atlas, optionally at a specific event offset. */
export async function loadAtlas(offset) {
  const query = offset === undefined || offset === null ? '' : `?offset=${encodeURIComponent(offset)}`;
  return api(`/atlas${query}`);
}

/** Load only a graph snapshot, used by the timeline scrubber. */
export async function loadGraph(offset) {
  const query = offset === undefined || offset === null ? '' : `?offset=${encodeURIComponent(offset)}`;
  return api(`/atlas/graph${query}`);
}

/** Load the node/edge type catalog. */
export function loadSchema() { return api('/atlas/schema'); }

/** Load the doctor report. */
export function loadHealth() { return api('/atlas/health'); }

/** Normalize raw atlas data into the shape the views render. */
export function normalize(raw) {
  const atlas = raw || {};
  const graph = atlas.graph || {};
  const nodes = Array.isArray(graph.nodes) ? graph.nodes : [];
  const edges = Array.isArray(graph.edges) ? graph.edges : [];
  const schema = atlas.schema || {};
  const coverage = atlas.coverage && typeof atlas.coverage === 'object' ? atlas.coverage : null;
  const chain = atlas.chain && typeof atlas.chain === 'object' ? atlas.chain : {};
  const status = atlas.status && typeof atlas.status === 'object' ? atlas.status : {};
  const sources = Array.isArray(atlas.sources) ? atlas.sources : [];

  return {
    project: atlas.project || { root: '', initialized: false },
    status,
    chain,
    graph: {
      offset: graph.offset ?? 0,
      eventCount: graph.event_count ?? 0,
      nodeCount: graph.node_count ?? nodes.length,
      edgeCount: graph.edge_count ?? edges.length,
      nodesReturned: graph.nodes_returned ?? nodes.length,
      edgesReturned: graph.edges_returned ?? edges.length,
      truncated: Boolean(graph.truncated),
      nodes,
      edges,
      chainHead: graph.chain_head ?? null,
    },
    schema: {
      nodeTypes: Array.isArray(schema.node_types) ? schema.node_types : [],
      edgeTypes: Array.isArray(schema.edge_types) ? schema.edge_types : [],
      contractCount: schema.contract_count ?? status.contracts ?? 0,
    },
    coverage,
    sources,
    timing: summarizeSources(sources),
  };
}

/** Aggregate provenance for the lineage footer. */
export function summarizeSources(sources) {
  const healthy = sources.filter((s) => s && s.ok !== false).length;
  const duration = sources.reduce((total, s) => total + (Number(s?.duration_ms) || 0), 0);
  return { total: sources.length, healthy, durationMs: duration };
}

/** Count graph nodes by type, stable for rendering and tests. */
export function nodeCounts(nodes) {
  const counts = new Map();
  for (const node of nodes || []) {
    const type = node?.type_id || 'Unknown';
    counts.set(type, (counts.get(type) || 0) + 1);
  }
  return [...counts.entries()].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));
}

/** Count graph edges by predicate. */
export function edgeCounts(edges) {
  const counts = new Map();
  for (const edge of edges || []) {
    const type = edge?.type_id || 'unknown';
    counts.set(type, (counts.get(type) || 0) + 1);
  }
  return [...counts.entries()].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));
}

/** Build an indexed graph model suitable for layout and rendering. */
export function graphModel(graph) {
  const nodes = (graph?.nodes || []).map((node, index) => ({
    id: String(node.id ?? `node-${index}`),
    type: String(node.type_id ?? 'Unknown'),
    state: String(node.state ?? ''),
    properties: node.properties || {},
    immutable: Boolean(node.immutable),
    tombstoned: Boolean(node.tombstoned),
    createdEvent: node.created_event ?? null,
  }));
  const byId = new Map(nodes.map((node, index) => [node.id, index]));
  const edges = (graph?.edges || [])
    .map((edge, index) => ({
      id: String(edge.id ?? `edge-${index}`),
      type: String(edge.type_id ?? 'unknown'),
      source: String(edge.source ?? ''),
      target: String(edge.target ?? ''),
      properties: edge.properties || {},
    }))
    .filter((edge) => byId.has(edge.source) && byId.has(edge.target))
    .map((edge) => ({ ...edge, sourceIndex: byId.get(edge.source), targetIndex: byId.get(edge.target) }));
  const degree = new Array(nodes.length).fill(0);
  for (const edge of edges) {
    degree[edge.sourceIndex] += 1;
    degree[edge.targetIndex] += 1;
  }
  return { nodes, edges, degree, byId };
}

/** Coverage summary from the CLI report, without inventing values. */
export function coverageSummary(coverage) {
  if (!coverage) return null;
  const total = Number(coverage.requirements_total) || 0;
  const verified = Number(coverage.requirements_verified) || 0;
  const gaps = Array.isArray(coverage.gaps) ? coverage.gaps : [];
  return {
    total,
    verified,
    gaps: gaps.length,
    percent: total ? Math.round((verified / total) * 100) : 0,
    claim: coverage.claim || '',
    rows: gaps,
  };
}

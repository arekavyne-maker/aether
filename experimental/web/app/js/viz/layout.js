// Deterministic graph layouts. No randomness, no external libraries.
// Every layout returns { [nodeId]: {x, y} } plus a stable bounds object.

/** FNV-1a hash, used only for stable initial placement and tie-breaking. */
export function hash(text) {
  let value = 0x811c9dc5;
  for (let i = 0; i < String(text).length; i += 1) {
    value ^= String(text).charCodeAt(i);
    value = Math.imul(value, 0x01000193);
  }
  return value >>> 0;
}

function unit(text) {
  return (hash(text) % 10000) / 10000;
}

function boundsOf(positions) {
  const values = Object.values(positions);
  if (!values.length) return { minX: 0, maxX: 0, minY: 0, maxY: 0, width: 0, height: 0 };
  const xs = values.map((p) => p.x);
  const ys = values.map((p) => p.y);
  const minX = Math.min(...xs);
  const maxX = Math.max(...xs);
  const minY = Math.min(...ys);
  const maxY = Math.max(...ys);
  return { minX, maxX, minY, maxY, width: maxX - minX || 1, height: maxY - minY || 1 };
}

function result(positions) {
  return { positions, bounds: boundsOf(positions) };
}

/** Circular fallback for large graphs; stable and cheap. */
export function circularLayout(model) {
  const nodes = model?.nodes || [];
  const positions = {};
  const radius = Math.max(160, nodes.length * 12);
  nodes.forEach((node, index) => {
    const angle = (index / Math.max(1, nodes.length)) * Math.PI * 2;
    positions[node.id] = { x: Math.cos(angle) * radius, y: Math.sin(angle) * radius };
  });
  return result(positions);
}

/** Layered DAG layout with Kahn ranks and barycenter ordering. */
export function layeredLayout(model) {
  const nodes = model?.nodes || [];
  const edges = model?.edges || [];
  if (!nodes.length) return result({});
  if (nodes.length > 4000) return circularLayout(model);

  const indegree = new Array(nodes.length).fill(0);
  const outgoing = nodes.map(() => []);
  for (const edge of edges) {
    indegree[edge.targetIndex] += 1;
    outgoing[edge.sourceIndex].push(edge.targetIndex);
  }
  const rank = new Array(nodes.length).fill(0);
  const queue = [];
  indegree.forEach((degree, index) => { if (degree === 0) queue.push(index); });
  let visited = 0;
  while (queue.length) {
    const index = queue.shift();
    visited += 1;
    for (const target of outgoing[index]) {
      rank[target] = Math.max(rank[target], rank[index] + 1);
      indegree[target] -= 1;
      if (indegree[target] === 0) queue.push(target);
    }
  }
  // Cyclic remnants get the next rank so they remain visible.
  indegree.forEach((degree, index) => { if (degree > 0) rank[index] = Math.max(rank[index], 1); });
  const layers = new Map();
  nodes.forEach((node, index) => {
    const r = rank[index];
    if (!layers.has(r)) layers.set(r, []);
    layers.get(r).push(index);
  });
  const ranks = [...layers.keys()].sort((a, b) => a - b);
  const positions = {};
  ranks.forEach((r, layerIndex) => {
    const members = layers.get(r).sort((a, b) => unit(nodes[a].id) - unit(nodes[b].id));
    members.forEach((index, at) => {
      positions[nodes[index].id] = {
        x: layerIndex * 190,
        y: (at - (members.length - 1) / 2) * 92,
      };
    });
  });
  return result(positions);
}

/** Force-directed layout with deterministic initial positions. */
export function forceLayout(model, options = {}) {
  const nodes = model?.nodes || [];
  const edges = model?.edges || [];
  if (!nodes.length) return result({});
  if (nodes.length > 900) return layeredLayout(model);

  const iterations = Math.max(20, Math.min(240, Number(options.iterations) || 160));
  const positions = {};
  nodes.forEach((node, index) => {
    const angle = (index / nodes.length) * Math.PI * 2;
    const wobble = 0.75 + unit(`${node.id}:r`) * 0.5;
    positions[node.id] = {
      x: Math.cos(angle) * 180 * wobble,
      y: Math.sin(angle) * 180 * wobble,
    };
  });
  const velocity = nodes.map((node) => ({ x: 0, y: 0 }));

  for (let step = 0; step < iterations; step += 1) {
    const cooling = 1 - step / iterations;
    for (let i = 0; i < nodes.length; i += 1) {
      for (let j = i + 1; j < nodes.length; j += 1) {
        const a = positions[nodes[i].id];
        const b = positions[nodes[j].id];
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let distance = Math.hypot(dx, dy);
        if (distance < 0.01) {
          dx = (unit(`${nodes[i].id}:x`) - 0.5) * 2;
          dy = (unit(`${nodes[j].id}:y`) - 0.5) * 2;
          distance = Math.hypot(dx, dy) || 1;
        }
        const push = Math.min(160, 24000 / (distance * distance)) * cooling;
        const fx = (dx / distance) * push;
        const fy = (dy / distance) * push;
        velocity[i].x -= fx;
        velocity[i].y -= fy;
        velocity[j].x += fx;
        velocity[j].y += fy;
      }
    }
    for (const edge of edges) {
      const a = positions[model.nodes[edge.sourceIndex].id];
      const b = positions[model.nodes[edge.targetIndex].id];
      const dx = b.x - a.x;
      const dy = b.y - a.y;
      const distance = Math.hypot(dx, dy) || 1;
      const pull = (distance - 150) * 0.012 * cooling;
      const fx = (dx / distance) * pull;
      const fy = (dy / distance) * pull;
      velocity[edge.sourceIndex].x += fx;
      velocity[edge.sourceIndex].y += fy;
      velocity[edge.targetIndex].x -= fx;
      velocity[edge.targetIndex].y -= fy;
    }
    for (let i = 0; i < nodes.length; i += 1) {
      const position = positions[nodes[i].id];
      // A weak gravity toward the origin keeps disconnected pieces together.
      velocity[i].x -= position.x * 0.0015;
      velocity[i].y -= position.y * 0.0015;
      velocity[i].x *= 0.82;
      velocity[i].y *= 0.82;
      position.x += Math.max(-18, Math.min(18, velocity[i].x));
      position.y += Math.max(-18, Math.min(18, velocity[i].y));
    }
  }
  return result(positions);
}

/** Radial layout centered on a focus node. */
export function radialLayout(model, focusId) {
  const nodes = model?.nodes || [];
  if (!nodes.length) return result({});
  const edges = model?.edges || [];
  const adjacency = nodes.map(() => []);
  for (const edge of edges) {
    adjacency[edge.sourceIndex].push(edge.targetIndex);
    adjacency[edge.targetIndex].push(edge.sourceIndex);
  }
  let focus = model.byId.get(String(focusId));
  if (focus === undefined) {
    focus = nodes.reduce((best, _node, index) => ((model.degree?.[index] ?? 0) > (model.degree?.[best] ?? 0) ? index : best), 0);
  }
  const depth = new Array(nodes.length).fill(-1);
  depth[focus] = 0;
  const queue = [focus];
  while (queue.length) {
    const current = queue.shift();
    for (const next of adjacency[current]) {
      if (depth[next] === -1) {
        depth[next] = depth[current] + 1;
        queue.push(next);
      }
    }
  }
  const groups = new Map();
  nodes.forEach((node, index) => {
    const level = depth[index] < 0 ? 1 : depth[index];
    if (!groups.has(level)) groups.set(level, []);
    groups.get(level).push(index);
  });
  const positions = {};
  for (const [level, members] of groups) {
    members.sort((a, b) => unit(nodes[a].id) - unit(nodes[b].id));
    members.forEach((index, at) => {
      const angle = (at / members.length) * Math.PI * 2 + level * 0.35;
      const radius = level === 0 ? 0 : level * 150;
      positions[nodes[index].id] = { x: Math.cos(angle) * radius, y: Math.sin(angle) * radius };
    });
  }
  return result(positions);
}

/** Map model coordinates into a viewport with padding. */
export function fit(positions, bounds, width, height, padding = 48) {
  const safeWidth = Math.max(1, width - padding * 2);
  const safeHeight = Math.max(1, height - padding * 2);
  const scale = Math.min(safeWidth / bounds.width, safeHeight / bounds.height, 2.5);
  const offsetX = padding + (safeWidth - bounds.width * scale) / 2 - bounds.minX * scale;
  const offsetY = padding + (safeHeight - bounds.height * scale) / 2 - bounds.minY * scale;
  const output = {};
  for (const [id, point] of Object.entries(positions)) {
    output[id] = { x: point.x * scale + offsetX, y: point.y * scale + offsetY };
  }
  return { positions: output, scale };
}

/** Compare two snapshots and classify nodes/edges. */
export function diffGraph(before, after) {
  const ids = (graph) => new Set((graph?.nodes || []).map((n) => String(n.id)));
  const edgeIds = (graph) => new Set((graph?.edges || []).map((e) => String(e.id)));
  const beforeNodes = ids(before);
  const afterNodes = ids(after);
  const beforeEdges = edgeIds(before);
  const afterEdges = edgeIds(after);
  return {
    addedNodes: [...afterNodes].filter((id) => !beforeNodes.has(id)).sort(),
    removedNodes: [...beforeNodes].filter((id) => !afterNodes.has(id)).sort(),
    addedEdges: [...afterEdges].filter((id) => !beforeEdges.has(id)).sort(),
    removedEdges: [...beforeEdges].filter((id) => !afterEdges.has(id)).sort(),
  };
}

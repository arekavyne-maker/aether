// Squarified treemap for command/workspace counts. Pure and testable.

/** Lay out weighted items in a rectangle. Returns flat rects with original refs. */
export function treemap(items, width = 900, height = 520, padding = 2) {
  const total = items.reduce((sum, item) => sum + Math.max(0, Number(item.value) || 0), 0);
  if (!total || !items.length) return [];
  const sorted = [...items].sort((a, b) => (b.value || 0) - (a.value || 0));
  const rects = [];
  const stack = [{ x: 0, y: 0, w: width, h: height, items: sorted }];

  while (stack.length) {
    const cell = stack.pop();
    if (!cell.items.length || cell.w <= 0 || cell.h <= 0) continue;
    const sum = cell.items.reduce((acc, item) => acc + Math.max(0, Number(item.value) || 0), 0);
    if (cell.items.length === 1) {
      const item = cell.items[0];
      rects.push({ ...item, x: cell.x, y: cell.y, w: cell.w, h: cell.h });
      continue;
    }
    const horizontal = cell.w >= cell.h;
    const side = horizontal ? cell.h : cell.w;
    const scale = sum ? side / sum : 0;
    let bestIndex = 1;
    let bestRatio = Infinity;
    for (let i = 1; i < cell.items.length; i += 1) {
      const first = (cell.items[0].value || 0) * scale;
      const rest = cell.items.slice(0, i + 1).reduce((acc, item) => acc + Math.max(0, item.value || 0) * scale, 0);
      const maxSide = Math.max(first, rest / i);
      const minSide = Math.min(rest / i, (sum - cell.items.slice(0, i + 1).reduce((acc, item) => acc + (item.value || 0), 0)) * scale);
      const ratio = maxSide && minSide ? Math.max(maxSide / minSide, minSide / maxSide) : Math.abs(maxSide - minSide);
      if (ratio < bestRatio) { bestRatio = ratio; bestIndex = i; }
    }
    const group = cell.items.slice(0, bestIndex + 1);
    const remainder = cell.items.slice(bestIndex + 1);
    const groupSum = group.reduce((acc, item) => acc + Math.max(0, item.value || 0), 0);
    const groupThickness = horizontal ? groupSum * (cell.w / sum) : groupSum * (cell.h / sum);
    const local = { x: cell.x, y: cell.y, w: cell.w, h: cell.h, items: group };
    const remaining = horizontal
      ? { x: cell.x + groupThickness, y: cell.y, w: cell.w - groupThickness, h: cell.h, items: remainder }
      : { x: cell.x, y: cell.y + groupThickness, w: cell.w, h: cell.h - groupThickness, items: remainder };
    stack.push(remaining);
    // Split the group perpendicularly.
    let offset = 0;
    const groupLength = horizontal ? cell.h : cell.w;
    group.forEach((item, index) => {
      const size = (Math.max(0, item.value || 0) / groupSum) * groupLength;
      const rect = horizontal
        ? { x: cell.x, y: cell.y + offset, w: groupThickness, h: size }
        : { x: cell.x + offset, y: cell.y, w: size, h: groupThickness };
      rects.push({
        ...item,
        x: rect.x + padding, y: rect.y + padding,
        w: Math.max(0, rect.w - padding * 2), h: Math.max(0, rect.h - padding * 2),
      });
      offset += size;
    });
  }
  return rects;
}

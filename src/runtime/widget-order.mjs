function uniqueKnownIds(ids, known) {
  const seen = new Set()
  return ids.filter(id => known.has(id) && !seen.has(id) && seen.add(id))
}

export function mergeVisibleOrder(existingOrder, visibleOrder, scopeIds) {
  const scopeSet = new Set(scopeIds)
  const canonical = uniqueKnownIds(existingOrder, scopeSet)
  const canonicalSet = new Set(canonical)
  canonical.push(...scopeIds.filter(id => !canonicalSet.has(id)))

  const visible = uniqueKnownIds(visibleOrder, scopeSet)
  const visibleSet = new Set(visible)
  let nextVisibleIndex = 0
  return canonical.map(id => (
    visibleSet.has(id) ? visible[nextVisibleIndex++] : id
  ))
}

export function applyWidgetOrder(tasks, order) {
  const rank = new Map(order.map((id, index) => [id, index]))
  return tasks
    .map((task, index) => ({ task, index }))
    .sort((a, b) => {
      const aRank = rank.get(a.task.id)
      const bRank = rank.get(b.task.id)
      if (aRank !== undefined && bRank !== undefined) return aRank - bRank
      if (aRank !== undefined) return -1
      if (bRank !== undefined) return 1
      return a.index - b.index
    })
    .map(item => item.task)
}

export function moveVisibleId(ids, draggedId, targetId) {
  const fromIndex = ids.indexOf(draggedId)
  const targetIndex = ids.indexOf(targetId)
  if (fromIndex < 0 || targetIndex < 0 || fromIndex === targetIndex) return [...ids]
  const next = [...ids]
  next.splice(fromIndex, 1)
  next.splice(Math.min(targetIndex, next.length), 0, draggedId)
  return next
}

export function hasExceededDragThreshold(startX, startY, currentX, currentY, threshold = 5) {
  return Math.hypot(currentX - startX, currentY - startY) > threshold
}

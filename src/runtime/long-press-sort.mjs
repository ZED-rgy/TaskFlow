import Sortable from 'sortablejs'

const clickGuards = new WeakMap()

// Let touch scrolling cancel the pending long press. Buttons outside the handle
// remain ordinary controls, including completion and the more menu.
export function createLongPressSort(element, { draggable, handle, onReorder }) {
  let original = []
  const guard = clickGuards.get(element) || { until: 0 }
  clickGuards.set(element, guard)
  const stopClick = event => {
    if (Date.now() < guard.until) { event.preventDefault(); event.stopImmediatePropagation() }
  }
  const stopMenu = event => event.preventDefault()
  element.addEventListener('click', stopClick, true)
  element.addEventListener('contextmenu', stopMenu)
  const sortable = Sortable.create(element, {
    draggable, handle, delay: 350, delayOnTouchOnly: true,
    touchStartThreshold: 8, fallbackTolerance: 5, supportPointer: false,
    animation: 150, forceFallback: true, fallbackOnBody: true,
    fallbackClass: 'long-press-preview', ghostClass: 'long-press-ghost', chosenClass: 'long-press-chosen',
    onStart() {
      original = [...element.childNodes].filter(node => !node.classList?.contains('long-press-preview'))
      guard.until = Infinity
      navigator.vibrate?.(15)
    },
    onEnd(event) {
      guard.until = Date.now() + 350
      const ids = [...element.children].filter(node => node.matches(draggable) && !node.classList.contains('long-press-preview')).map(node => node.dataset.id)
      // Restore the framework-owned DOM before Vue applies the new model order.
      const next = original.slice(original.indexOf(event.item) + 1).find(node => node.parentNode === element)
      if (event.item.parentNode === element) element.insertBefore(event.item, next || null)
      if (event.oldDraggableIndex !== event.newDraggableIndex) onReorder(ids)
    },
  })
  return { destroy() {
    sortable.destroy()
    if (!Number.isFinite(guard.until)) guard.until = Date.now() + 350
    element.removeEventListener('click', stopClick, true)
    element.removeEventListener('contextmenu', stopMenu)
  } }
}

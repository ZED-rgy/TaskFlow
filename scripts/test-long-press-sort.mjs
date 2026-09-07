import assert from 'node:assert/strict'
import fs from 'node:fs'

const source = fs.readFileSync(new URL('../src/runtime/long-press-sort.mjs', import.meta.url), 'utf8').replace(/^import[^\n]*\n/, '').replace('export function', 'function')
let options, destroyed = false
const factory = new Function('Sortable', `${source}; return createLongPressSort`)({ create: (_, value) => { options = value; return { destroy: () => { destroyed = true } } } })
const row = id => ({ dataset: { id }, classList: { contains: () => false }, matches: () => true })
const a = row('a'), b = row('b'), anchor = { nodeType: 3 }
const listeners = new Map()
const element = { childNodes: [a,b,anchor], get children() { return this.childNodes.filter(node => node !== anchor) }, addEventListener: (type, fn) => listeners.set(type,fn), removeEventListener: type => listeners.delete(type), insertBefore(node, next) { this.childNodes = this.childNodes.filter(item => item !== node); const index = next ? this.childNodes.indexOf(next) : this.childNodes.length; this.childNodes.splice(index,0,node) } }
for (const node of element.childNodes) node.parentNode = element
let saved
const sort = factory(element, { draggable: '.row', handle: '.title', onReorder: ids => { saved = ids } })
assert.equal(options.delayOnTouchOnly,true)
assert.equal(options.delay,350)
assert.equal(options.touchStartThreshold,8)
assert.equal(options.handle,'.title', 'completion and menu buttons are outside the drag handle')
options.onStart()
element.childNodes = [b,a,anchor]
options.onEnd({item:a,oldDraggableIndex:0,newDraggableIndex:1})
assert.deepEqual(saved,['b','a'])
assert.deepEqual(element.childNodes,[a,b,anchor], 'preserve Vue fragment anchors so the next new task appends in the right place')
let prevented = false
listeners.get('click')({ preventDefault:()=>{prevented=true},stopImmediatePropagation(){} })
assert.ok(prevented, 'release after drag cannot open task details')
saved = null
options.onStart(); options.onEnd({item:a,oldDraggableIndex:0,newDraggableIndex:0})
assert.equal(saved,null,'stationary long press does not persist an order')
sort.destroy()
assert.ok(destroyed)
assert.equal(listeners.size,0)
const replacement = factory(element, {draggable:'.row', handle:'.title', onReorder(){} })
prevented = false
listeners.get('click')({ preventDefault:()=>{prevented=true},stopImmediatePropagation(){} })
assert.ok(prevented,'Vue refresh after reorder preserves the release-click guard')
replacement.destroy()
console.log('long press sort: touch delay, control isolation, DOM restore and cleanup passed')

const interrupted = factory(element, {draggable:'.row',handle:'.title',onReorder(){}})
options.onStart()
interrupted.destroy()
const actualNow = Date.now
Date.now = () => actualNow() + 1000
const recovered = factory(element, {draggable:'.row',handle:'.title',onReorder(){}})
prevented = false
listeners.get('click')({preventDefault:()=>{prevented=true},stopImmediatePropagation(){}})
assert.equal(prevented,false,'interrupted drag must not block all future clicks')
Date.now = actualNow
recovered.destroy()

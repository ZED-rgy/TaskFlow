const fs = require('fs')
const path = require('path')

const iconSizes = [16, 24, 32, 48, 64, 128, 256]
const aaScale = 4

const rgba = (hex, alpha = 255) => {
  const clean = hex.replace('#', '')
  return [
    parseInt(clean.slice(0, 2), 16),
    parseInt(clean.slice(2, 4), 16),
    parseInt(clean.slice(4, 6), 16),
    alpha,
  ]
}

function makeCanvas(size) {
  const canvasSize = size * aaScale
  return {
    size,
    canvasSize,
    data: new Float32Array(canvasSize * canvasSize * 4),
  }
}

function toPx(canvas, value) {
  return (value / 256) * canvas.size * aaScale
}

function blendPixel(canvas, x, y, color) {
  if (x < 0 || x >= canvas.canvasSize || y < 0 || y >= canvas.canvasSize) return
  const offset = (y * canvas.canvasSize + x) * 4
  const srcA = color[3] / 255
  const dstA = canvas.data[offset + 3]
  const outA = srcA + dstA * (1 - srcA)
  if (outA <= 0) return
  canvas.data[offset] = (color[0] * srcA + canvas.data[offset] * dstA * (1 - srcA)) / outA
  canvas.data[offset + 1] = (color[1] * srcA + canvas.data[offset + 1] * dstA * (1 - srcA)) / outA
  canvas.data[offset + 2] = (color[2] * srcA + canvas.data[offset + 2] * dstA * (1 - srcA)) / outA
  canvas.data[offset + 3] = outA
}

function roundedRect(canvas, x, y, w, h, radius, color) {
  const sx = Math.round(toPx(canvas, x))
  const sy = Math.round(toPx(canvas, y))
  const sw = Math.round(toPx(canvas, w))
  const sh = Math.round(toPx(canvas, h))
  const sr = toPx(canvas, radius)
  for (let py = sy; py < sy + sh; py += 1) {
    for (let px = sx; px < sx + sw; px += 1) {
      const cx = Math.min(Math.max(px, sx + sr), sx + sw - sr - 1)
      const cy = Math.min(Math.max(py, sy + sr), sy + sh - sr - 1)
      if (Math.hypot(px - cx, py - cy) <= sr) blendPixel(canvas, px, py, color)
    }
  }
}

function circle(canvas, cx, cy, radius, color) {
  const scx = toPx(canvas, cx)
  const scy = toPx(canvas, cy)
  const sr = toPx(canvas, radius)
  for (let py = Math.floor(scy - sr); py <= Math.ceil(scy + sr); py += 1) {
    for (let px = Math.floor(scx - sr); px <= Math.ceil(scx + sr); px += 1) {
      if (Math.hypot(px - scx, py - scy) <= sr) blendPixel(canvas, px, py, color)
    }
  }
}

function line(canvas, x1, y1, x2, y2, width, color) {
  const sx1 = toPx(canvas, x1)
  const sy1 = toPx(canvas, y1)
  const sx2 = toPx(canvas, x2)
  const sy2 = toPx(canvas, y2)
  const radius = toPx(canvas, width / 2)
  const steps = Math.max(Math.abs(sx2 - sx1), Math.abs(sy2 - sy1)) * 2
  for (let i = 0; i <= steps; i += 1) {
    const t = i / steps
    const x = sx1 + (sx2 - sx1) * t
    const y = sy1 + (sy2 - sy1) * t
    for (let py = Math.floor(y - radius); py <= Math.ceil(y + radius); py += 1) {
      for (let px = Math.floor(x - radius); px <= Math.ceil(x + radius); px += 1) {
        if (Math.hypot(px - x, py - y) <= radius) blendPixel(canvas, px, py, color)
      }
    }
  }
}

function drawIcon(size) {
  const canvas = makeCanvas(size)
  roundedRect(canvas, 8, 8, 240, 240, 56, rgba('#111827'))
  roundedRect(canvas, 14, 14, 228, 228, 50, rgba('#172033', 180))

  circle(canvas, 128, 126, 74, rgba('#F2B84B'))
  if (size >= 64) {
    circle(canvas, 109, 103, 24, rgba('#FFE7A0', 120))
  }

  line(canvas, 78, 128, 114, 165, 30, rgba('#111827'))
  line(canvas, 114, 165, 179, 86, 30, rgba('#111827'))

  return canvas
}

function canvasToDib(canvas) {
  const { size, canvasSize, data } = canvas
  const pixels = Buffer.alloc(size * size * 4)
  const mask = Buffer.alloc(Math.ceil(size / 32) * 4 * size)

  for (let y = 0; y < size; y += 1) {
    for (let x = 0; x < size; x += 1) {
      let r = 0
      let g = 0
      let b = 0
      let a = 0
      for (let oy = 0; oy < aaScale; oy += 1) {
        for (let ox = 0; ox < aaScale; ox += 1) {
          const offset = (((y * aaScale + oy) * canvasSize) + (x * aaScale + ox)) * 4
          const alpha = data[offset + 3]
          r += data[offset] * alpha
          g += data[offset + 1] * alpha
          b += data[offset + 2] * alpha
          a += alpha
        }
      }

      const samples = aaScale * aaScale
      const outA = a / samples
      const row = size - 1 - y
      const out = (row * size + x) * 4
      pixels[out] = outA ? Math.round(b / a) : 0
      pixels[out + 1] = outA ? Math.round(g / a) : 0
      pixels[out + 2] = outA ? Math.round(r / a) : 0
      pixels[out + 3] = Math.round(outA * 255)
    }
  }

  const bitmapHeader = Buffer.alloc(40)
  bitmapHeader.writeUInt32LE(40, 0)
  bitmapHeader.writeInt32LE(size, 4)
  bitmapHeader.writeInt32LE(size * 2, 8)
  bitmapHeader.writeUInt16LE(1, 12)
  bitmapHeader.writeUInt16LE(32, 14)
  bitmapHeader.writeUInt32LE(0, 16)
  bitmapHeader.writeUInt32LE(pixels.length + mask.length, 20)

  return Buffer.concat([bitmapHeader, pixels, mask])
}

const images = iconSizes.map(size => ({ size, image: canvasToDib(drawIcon(size)) }))
const header = Buffer.alloc(6)
header.writeUInt16LE(0, 0)
header.writeUInt16LE(1, 2)
header.writeUInt16LE(images.length, 4)

const directories = []
let offset = header.length + images.length * 16
for (const { size, image } of images) {
  const directory = Buffer.alloc(16)
  directory.writeUInt8(size === 256 ? 0 : size, 0)
  directory.writeUInt8(size === 256 ? 0 : size, 1)
  directory.writeUInt8(0, 2)
  directory.writeUInt8(0, 3)
  directory.writeUInt16LE(1, 4)
  directory.writeUInt16LE(32, 6)
  directory.writeUInt32LE(image.length, 8)
  directory.writeUInt32LE(offset, 12)
  directories.push(directory)
  offset += image.length
}

const icon = Buffer.concat([header, ...directories, ...images.map(item => item.image)])
const outputPaths = [
  path.join(__dirname, '..', 'assets', 'icon.ico'),
  path.join(__dirname, '..', 'public', 'favicon.ico'),
]

for (const outputPath of outputPaths) {
  fs.mkdirSync(path.dirname(outputPath), { recursive: true })
  fs.writeFileSync(outputPath, icon)
  console.log(`Generated ${outputPath}`)
}

// Android and Windows share drawIcon; no independently maintained logo assets.
const zlib = require('zlib')
function pngChunk(type, data) {
  const body = Buffer.concat([Buffer.from(type), data])
  let crc = 0xffffffff
  for (const byte of body) {
    crc ^= byte
    for (let bit = 0; bit < 8; bit++) crc = (crc >>> 1) ^ ((crc & 1) ? 0xedb88320 : 0)
  }
  const size = Buffer.alloc(4), checksum = Buffer.alloc(4)
  size.writeUInt32BE(data.length)
  checksum.writeUInt32BE((crc ^ 0xffffffff) >>> 0)
  return Buffer.concat([size, body, checksum])
}
function iconPng(size, canvasSize = size) {
  const dib = canvasToDib(drawIcon(size))
  const rows = Buffer.alloc((canvasSize * 4 + 1) * canvasSize)
  const inset = Math.floor((canvasSize - size) / 2)
  for (let y = 0; y < size; y++) for (let x = 0; x < size; x++) {
    const src = 40 + ((size - 1 - y) * size + x) * 4
    const dst = (y + inset) * (canvasSize * 4 + 1) + 1 + (x + inset) * 4
    rows[dst] = dib[src + 2]
    rows[dst + 1] = dib[src + 1]
    rows[dst + 2] = dib[src]
    rows[dst + 3] = dib[src + 3]
  }
  const ihdr = Buffer.alloc(13)
  ihdr.writeUInt32BE(canvasSize, 0); ihdr.writeUInt32BE(canvasSize, 4)
  ihdr[8] = 8; ihdr[9] = 6
  return Buffer.concat([Buffer.from('89504e470d0a1a0a', 'hex'), pngChunk('IHDR', ihdr), pngChunk('IDAT', zlib.deflateSync(rows)), pngChunk('IEND', Buffer.alloc(0))])
}
const root = path.join(__dirname, '..')
fs.writeFileSync(path.join(root, 'src-tauri', 'icons', 'icon.png'), iconPng(256))
const res = path.join(root, 'src-tauri', 'gen', 'android', 'app', 'src', 'main', 'res')
for (const [density, scale] of Object.entries({ mdpi: 1, hdpi: 1.5, xhdpi: 2, xxhdpi: 3, xxxhdpi: 4 })) {
  const dir = path.join(res, `mipmap-${density}`)
  fs.mkdirSync(dir, { recursive: true })
  for (const name of ['ic_launcher', 'ic_launcher_round']) fs.writeFileSync(path.join(dir, `${name}.png`), iconPng(48 * scale))
  // Keep the complete mark inside the adaptive icon's 66dp safe zone.
  fs.writeFileSync(path.join(dir, 'ic_launcher_foreground.png'), iconPng(64 * scale, 108 * scale))
}
const adaptiveDir = path.join(res, 'mipmap-anydpi-v26')
fs.mkdirSync(adaptiveDir, { recursive: true })
for (const name of ['ic_launcher', 'ic_launcher_round']) {
  fs.writeFileSync(path.join(adaptiveDir, `${name}.xml`), '<?xml version="1.0" encoding="utf-8"?>\n<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">\n    <background android:drawable="@color/taskflow_icon_background" />\n    <foreground android:drawable="@mipmap/ic_launcher_foreground" />\n</adaptive-icon>\n')
}
fs.mkdirSync(path.join(res, 'values'), { recursive: true })
fs.writeFileSync(path.join(res, 'values', 'taskflow_icon.xml'), '<?xml version="1.0" encoding="utf-8"?>\n<resources><color name="taskflow_icon_background">#111827</color></resources>\n')
console.log('Generated unified Android launcher icons and adaptive safe-zone foregrounds')

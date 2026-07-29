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

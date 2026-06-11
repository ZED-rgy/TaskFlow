const fs = require('fs')
const path = require('path')

const size = 256
const hotspot = size / 2
const pixels = Buffer.alloc(size * size * 4)
const mask = Buffer.alloc(Math.ceil(size / 32) * 4 * size)

function setPixel(x, y, r, g, b, a = 255) {
  const row = size - 1 - y
  const offset = (row * size + x) * 4
  pixels[offset] = b
  pixels[offset + 1] = g
  pixels[offset + 2] = r
  pixels[offset + 3] = a
}

function roundedRect(x, y, w, h, radius, color) {
  for (let py = y; py < y + h; py += 1) {
    for (let px = x; px < x + w; px += 1) {
      const dx = Math.max(x - px, 0, px - (x + w - 1))
      const dy = Math.max(y - py, 0, py - (y + h - 1))
      const cornerX = px < x + radius ? x + radius : px >= x + w - radius ? x + w - radius - 1 : px
      const cornerY = py < y + radius ? y + radius : py >= y + h - radius ? y + h - radius - 1 : py
      const dist = Math.hypot(px - cornerX, py - cornerY)
      if ((dx === 0 && dy === 0) || dist <= radius) setPixel(px, py, ...color)
    }
  }
}

function line(x1, y1, x2, y2, width, color) {
  const steps = Math.max(Math.abs(x2 - x1), Math.abs(y2 - y1)) * 2
  for (let i = 0; i <= steps; i += 1) {
    const t = i / steps
    const x = x1 + (x2 - x1) * t
    const y = y1 + (y2 - y1) * t
    for (let py = Math.floor(y - width); py <= Math.ceil(y + width); py += 1) {
      for (let px = Math.floor(x - width); px <= Math.ceil(x + width); px += 1) {
        if (px >= 0 && px < size && py >= 0 && py < size && Math.hypot(px - x, py - y) <= width) {
          setPixel(px, py, ...color)
        }
      }
    }
  }
}

function polygon(points, strokeWidth, color) {
  for (let i = 0; i < points.length; i += 1) {
    const [x1, y1] = points[i]
    const [x2, y2] = points[(i + 1) % points.length]
    line(x1, y1, x2, y2, strokeWidth, color)
  }
}

roundedRect(0, 0, size, size, 56, [17, 19, 24, 255])
roundedRect(32, 32, 192, 192, 40, [24, 26, 32, 255])
polygon([[128, 48], [198, 92], [198, 164], [128, 208], [58, 164], [58, 92]], 8.5, [212, 146, 42, 255])
line(92, 128, 120, 158, 12, [234, 231, 225, 255])
line(120, 158, 174, 96, 12, [234, 231, 225, 255])
roundedRect(64, 64, 20, 20, 8, [91, 142, 192, 255])
roundedRect(172, 172, 20, 20, 8, [94, 158, 114, 255])

const bitmapHeader = Buffer.alloc(40)
bitmapHeader.writeUInt32LE(40, 0)
bitmapHeader.writeInt32LE(size, 4)
bitmapHeader.writeInt32LE(size * 2, 8)
bitmapHeader.writeUInt16LE(1, 12)
bitmapHeader.writeUInt16LE(32, 14)
bitmapHeader.writeUInt32LE(0, 16)
bitmapHeader.writeUInt32LE(pixels.length + mask.length, 20)

const image = Buffer.concat([bitmapHeader, pixels, mask])
const header = Buffer.alloc(6)
header.writeUInt16LE(0, 0)
header.writeUInt16LE(1, 2)
header.writeUInt16LE(1, 4)

const directory = Buffer.alloc(16)
directory.writeUInt8(0, 0)
directory.writeUInt8(0, 1)
directory.writeUInt8(0, 2)
directory.writeUInt8(0, 3)
directory.writeUInt16LE(1, 4)
directory.writeUInt16LE(32, 6)
directory.writeUInt32LE(image.length, 8)
directory.writeUInt32LE(header.length + directory.length, 12)

const outPath = path.join(__dirname, '..', 'assets', 'icon.ico')
fs.writeFileSync(outPath, Buffer.concat([header, directory, image]))
console.log(`Generated ${outPath}`)

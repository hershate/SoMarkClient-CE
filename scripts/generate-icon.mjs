import { mkdirSync, writeFileSync } from 'node:fs'
import { deflateSync } from 'node:zlib'

const size = 1024
const pixels = Buffer.alloc(size * size * 4)
const crcTable = Array.from({ length: 256 }, (_, value) => {
  let crc = value
  for (let bit = 0; bit < 8; bit += 1) crc = (crc >>> 1) ^ (crc & 1 ? 0xedb88320 : 0)
  return crc >>> 0
})

function crc32(buffer) {
  let crc = 0xffffffff
  for (const byte of buffer) crc = crcTable[(crc ^ byte) & 0xff] ^ (crc >>> 8)
  return (crc ^ 0xffffffff) >>> 0
}

function chunk(type, data) {
  const name = Buffer.from(type)
  const length = Buffer.alloc(4)
  const checksum = Buffer.alloc(4)
  length.writeUInt32BE(data.length)
  checksum.writeUInt32BE(crc32(Buffer.concat([name, data])))
  return Buffer.concat([length, name, data, checksum])
}

function roundedRect(x, y, width, height, radius, color) {
  const edge = 1.25
  for (let py = Math.max(0, Math.floor(y)); py < Math.min(size, Math.ceil(y + height)); py += 1) {
    for (let px = Math.max(0, Math.floor(x)); px < Math.min(size, Math.ceil(x + width)); px += 1) {
      const cx = Math.max(x + radius, Math.min(px + 0.5, x + width - radius))
      const cy = Math.max(y + radius, Math.min(py + 0.5, y + height - radius))
      const distance = Math.hypot(px + 0.5 - cx, py + 0.5 - cy)
      const alpha = Math.max(0, Math.min(1, radius + 0.5 - distance / edge))
      if (alpha <= 0) continue
      const offset = (py * size + px) * 4
      for (let channel = 0; channel < 3; channel += 1) {
        pixels[offset + channel] = Math.round(pixels[offset + channel] * (1 - alpha) + color[channel] * alpha)
      }
      pixels[offset + 3] = Math.round(pixels[offset + 3] * (1 - alpha) + 255 * alpha)
    }
  }
}

roundedRect(32, 32, 960, 960, 220, [17, 111, 85])
roundedRect(270, 240, 484, 108, 54, [247, 245, 236])
roundedRect(270, 240, 108, 296, 54, [247, 245, 236])
roundedRect(270, 428, 484, 108, 54, [247, 245, 236])
roundedRect(646, 428, 108, 336, 54, [247, 245, 236])
roundedRect(270, 656, 484, 108, 54, [247, 245, 236])
roundedRect(718, 718, 92, 92, 46, [223, 169, 74])

const header = Buffer.alloc(13)
header.writeUInt32BE(size, 0)
header.writeUInt32BE(size, 4)
header[8] = 8
header[9] = 6
const scanlines = Buffer.alloc((size * 4 + 1) * size)
for (let row = 0; row < size; row += 1) {
  const target = row * (size * 4 + 1)
  scanlines[target] = 0
  pixels.copy(scanlines, target + 1, row * size * 4, (row + 1) * size * 4)
}

mkdirSync('assets', { recursive: true })
writeFileSync('assets/somarkclient-icon.png', Buffer.concat([
  Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
  chunk('IHDR', header),
  chunk('IDAT', deflateSync(scanlines, { level: 9 })),
  chunk('IEND', Buffer.alloc(0)),
]))

const fs = require('fs')
const path = require('path')

const root = path.resolve(__dirname, '..')
const source = path.join(root, 'src-tauri', 'target', 'release', '小光任务.exe')
const outputDir = path.join(root, 'release')
const output = path.join(outputDir, '小光任务.exe')

if (!fs.existsSync(source)) {
  throw new Error(`Lite executable not found: ${source}`)
}

fs.mkdirSync(outputDir, { recursive: true })
fs.copyFileSync(source, output)
console.log(`Copied ${output}`)

const fs = require('fs')
const path = require('path')

const root = path.resolve(__dirname, '..')
const executableName = '小光任务.exe'
const source = path.join(root, 'src-tauri', 'target', 'release', executableName)
const outputDir = path.join(root, 'release')
const output = path.join(outputDir, executableName)

if (!fs.existsSync(source)) {
  throw new Error(`Lite executable not found: ${source}`)
}

fs.mkdirSync(outputDir, { recursive: true })
fs.copyFileSync(source, output)
console.log(`Copied ${output}`)

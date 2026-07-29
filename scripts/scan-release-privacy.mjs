import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'

const projectRoot = path.resolve(import.meta.dirname, '..')
const artifact = path.resolve(process.argv[2] || path.join(projectRoot, 'release', '小光任务.exe'))
const userHome = os.homedir()
const username = path.basename(userHome)

if (!fs.existsSync(artifact)) {
  throw new Error(`Release artifact not found: ${artifact}`)
}

const candidates = [
  ['workspace path', projectRoot],
  ['workspace path with forward slashes', projectRoot.replaceAll('\\', '/')],
  ['user home', userHome],
  ['user home with forward slashes', userHome.replaceAll('\\', '/')],
  ['Windows username', username],
].filter(([, value]) => value && value.length >= 4)

const encodings = ['utf8', 'utf16le']
const contents = fs.readFileSync(artifact)
const hits = []

for (const [label, value] of candidates) {
  for (const encoding of encodings) {
    if (contents.indexOf(Buffer.from(value, encoding)) !== -1) {
      hits.push(`${label} (${encoding})`)
    }
  }
}

if (hits.length) {
  console.error(`Privacy scan failed for ${path.basename(artifact)}:`)
  for (const hit of hits) console.error(`- ${hit}`)
  process.exitCode = 1
} else {
  console.log(`Privacy scan passed: ${path.basename(artifact)} (0 sensitive path matches)`)
}

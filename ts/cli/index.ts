import { spawnSync } from 'node:child_process'
import { existsSync } from 'node:fs'

import { host } from './targets.ts'

const bin = `${import.meta.dirname}/${host()}/opi`

if (!existsSync(bin)) {
  console.error(`Binary not found for ${host()}`)
  process.exit(1)
}

const result = spawnSync(bin, process.argv.slice(2), { stdio: 'inherit' })
process.exit(result.status ?? 0)

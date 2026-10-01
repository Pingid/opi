import { defineConfig } from 'vite-plus'

import { host, targets } from './ts/cli/targets.ts'

const crossed = targets.filter((target) => target !== host())
const builds = [
  ...crossed.map((target) => ({ target, build: `cargo zigbuild --target ${target}`, output: `./target/${target}` })),
  { target: host(), build: `cargo build --target ${host()}`, output: './target' },
]

export default defineConfig({
  pack: {
    entry: ['./ts/src/index.ts', './ts/cli/index.ts'],
    outDir: './dist',
  },
  run: {
    tasks: {
      publish: { command: 'node bin/pkg.ts', dependsOn: ['build'] },
      build: { command: ['vp pack', 'vp run compile:all'] },
      'compile:all': {
        command: builds.map((build) => `cp ${build.output}/release/opi ./dist/cli/opi-${build.target}`),
        dependsOn: builds.map((build) => `compile:${build.target}`),
      },
      ...builds
        .map((build) => ({
          [`compile:${build.target}`]: {
            command: `${build.build} --release`,
            cache: {
              input: ['./src/**/*.rs', 'Cargo.lock'],
              output: [`${build.output}/release/opi`],
            },
          },
        }))
        .reduce((acc, build) => Object.assign(acc, build), {}),
    },
  },
})

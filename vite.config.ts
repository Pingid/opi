import { defineConfig } from 'vite-plus'

import { host, targets } from './ts/cli/targets.ts'

const crossed = targets.filter((target) => target !== host())
const builds = [
  ...crossed.map((target) => ({ target, build: `cross build --target ${target}`, output: `./target/${target}` })),
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
      build: {
        command: builds.map((build) => `cp ${build.output}/release/opi ./dist/cli/opi-${build.target}`),
        dependsOn: ['compile:typescript', ...builds.map((build) => `compile:${build.target}`)],
      },
      'compile:typescript': { command: 'vp pack' },
      ...builds
        .map((build) => ({
          [`compile:${build.target}`]: {
            command: `${build.build} --release --no-default-features`,
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

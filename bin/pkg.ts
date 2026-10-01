import { Shell, Repo } from '@pingid/lib/workspace'
import { Arg, Cli, Cmd } from '@pingid/lib/api/cli'
import { join } from 'node:path'

const root = (...args: string[]) => join(import.meta.dirname, '..', ...args)

const pkgbranch = async (p: { branch: string; outDir: string }) => {
  const repo = await Repo.discover()

  await using tree = await repo.worktree(p.branch, { path: root('.cache/.worktrees', p.branch) })

  await Shell.io`find ${tree.dir} -mindepth 1 -maxdepth 1 ! -name .git -exec rm -rf {} +`
  await Shell.io`cp -R ${repo.dir}/${p.outDir} ${tree.dir}`
  await Shell.io`cp ${repo.dir}/package.json ${tree.dir}`

  await tree.add()

  // Skip when the staged tree matches the last build.
  if (!(await tree.staged())) return console.log('No changes; skipping build')

  const tags = await tree.tags('build-*')
  const last = tags.map((t) => parseInt(t.slice('build-'.length), 10)).filter(Number.isFinite)
  const tag = `build-${Math.max(0, ...last) + 1}`

  // The branch carries one flattened commit, so each build amends it rather than stacking.
  await tree.commit().amend().no_edit()
  await tree.tag(tag)
  await tree.push('origin', `HEAD:${p.branch}`).force()
  await tree.push('origin', tag)

  console.log(`Built and pushed to ${p.branch} and tagged ${tag}`)
}

Cli.run(
  Cmd.build('pkg')
    .arg(Arg.string('branch', { default: 'pkg' }))
    .handle(({ branch }) => pkgbranch({ branch, outDir: 'dist' })),
)

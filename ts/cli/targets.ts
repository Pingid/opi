export const targets = ['x86_64-unknown-linux-musl', 'aarch64-apple-darwin']

export const host = (options = { musl: true }) => {
  const archMap = { x64: 'x86_64', arm64: 'aarch64', arm: 'armv7' }
  const arch = archMap[process.arch as keyof typeof archMap]
  const platforms = {
    darwin: `${arch}-apple-darwin`,
    linux: `${arch}-unknown-linux-${options.musl ? 'musl' : 'gnu'}`,
    win32: `${arch}-pc-windows-msvc`,
    freebsd: `${arch}-unknown-freebsd`,
  }
  return platforms[process.platform as keyof typeof platforms]
}

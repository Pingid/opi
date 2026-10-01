import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
//#region ts/cli/targets.ts
const host = (options = { musl: true }) => {
	const arch = {
		x64: "x86_64",
		arm64: "aarch64",
		arm: "armv7"
	}[process.arch];
	return {
		darwin: `${arch}-apple-darwin`,
		linux: `${arch}-unknown-linux-${options.musl ? "musl" : "gnu"}`,
		win32: `${arch}-pc-windows-msvc`,
		freebsd: `${arch}-unknown-freebsd`
	}[process.platform];
};
//#endregion
//#region ts/cli/index.ts
const bin = `${import.meta.dirname}/${host()}/opi`;
if (!existsSync(bin)) {
	console.error(`Binary not found for ${host()}`);
	process.exit(1);
}
const result = spawnSync(bin, process.argv.slice(2), { stdio: "inherit" });
process.exit(result.status ?? 0);
//#endregion
export {};

//# sourceMappingURL=index.mjs.map
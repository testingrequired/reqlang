import { build } from "esbuild";

export const buildConfig = {
  entryPoints: ["extension.ts"],
  bundle: true,
  outfile: "out/extension.js",
  external: ["vscode"],
  format: "cjs",
  platform: "node",
  sourcemap: true,
};

await build(buildConfig);

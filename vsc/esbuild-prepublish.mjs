import { build } from "esbuild";
import { buildConfig } from "./esbuild-build.mjs";

/**
 * @type {import("esbuild").BuildOptions}
 */
export const prepublishConfig = {
  minify: true,
};

await build(Object.assign({}, buildConfig, prepublishConfig));

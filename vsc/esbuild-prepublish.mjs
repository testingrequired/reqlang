import { build } from "esbuild";
import { buildConfig } from "./esbuild-build.mjs";

export const prepublishConfig = Object.assign(
  {
    minify: true,
  },
  buildConfig
);

await build(prepublishConfig);

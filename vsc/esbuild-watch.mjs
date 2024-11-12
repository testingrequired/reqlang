import { context } from "esbuild";
import { buildConfig } from "./esbuild-build.mjs";

const ctx = await context(buildConfig);

await ctx.watch();

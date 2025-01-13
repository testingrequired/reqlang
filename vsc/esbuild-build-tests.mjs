import { context } from "esbuild";

async function main() {
  const ctx = await context({
    entryPoints: ["**/*.test.ts"],
    format: "cjs",
    minify: false,
    sourcemap: true,
    sourcesContent: true,
    platform: "node",
    outdir: "out",
    logLevel: "silent",
    treeShaking: true,
    plugins: [
      /* add to the end of plugins array */
      esbuildProblemMatcherPlugin,
    ],
  });

  await ctx.rebuild();
  await ctx.dispose();
}

/**
 * @type {import('esbuild').Plugin}
 */
const esbuildProblemMatcherPlugin = {
  name: "esbuild-problem-matcher",

  setup(build) {
    build.onStart(() => {
      console.log("[watch] build started");
    });
    build.onEnd((result) => {
      result.errors.forEach(({ text, location }) => {
        console.error(`✘ [ERROR] ${text}`);
        console.error(
          `    ${location.file}:${location.line}:${location.column}:`,
        );
      });
      console.log("[watch] build finished");
    });
  },
};

main().catch((e) => {
  console.error(e);
  process.exit(1);
});

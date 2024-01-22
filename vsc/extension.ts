"use strict";

import { ExtensionContext, tasks, commands, env, Uri } from "vscode";
import { ReqlangTaskProvider } from "./reqlangTaskProvider";

export function activate(context: ExtensionContext) {
  context.subscriptions.push(
    commands.registerCommand("reqlang.openMdnDocsHttp", () => {
      env.openExternal(
        Uri.parse("https://developer.mozilla.org/en-US/docs/Web/HTTP")
      );
    }),
    commands.registerCommand("reqlang.openMdnDocsHttpMessages", () => {
      env.openExternal(
        Uri.parse("https://developer.mozilla.org/en-US/docs/Web/HTTP/Messages")
      );
    }),
    commands.registerCommand("reqlang.openMdnDocsHttpSpecs", () => {
      env.openExternal(
        Uri.parse(
          "https://developer.mozilla.org/en-US/docs/Web/HTTP/Resources_and_specifications"
        )
      );
    })
  );

  tasks.registerTaskProvider("reqlang", new ReqlangTaskProvider());
}

export function deactivate() {}

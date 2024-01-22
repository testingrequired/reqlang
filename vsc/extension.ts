"use strict";

import { ExtensionContext, tasks, commands, env, Uri } from "vscode";
import { ReqlangTaskProvider } from "./reqlangTaskProvider";

export function activate(context: ExtensionContext) {
  context.subscriptions.push(
    commands.registerCommand("reqlang.openMdnDocs", () => {
      env.openExternal(
        Uri.parse("https://developer.mozilla.org/en-US/docs/Web/HTTP/Messages")
      );
    })
  );

  tasks.registerTaskProvider("reqlang", new ReqlangTaskProvider());
}

export function deactivate() {}

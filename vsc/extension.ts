"use strict";

import { ExtensionContext, tasks, commands, env, Uri, workspace } from "vscode";
import { ReqlangTaskProvider } from "./reqlangTaskProvider";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";

let lc: LanguageClient;

export function activate(context: ExtensionContext) {
  const serverOptions: ServerOptions = {
    command: "reqlang-lsp",
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      {
        language: "reqlang",
      },
    ],
    synchronize: {
      fileEvents: workspace.createFileSystemWatcher("**/*.reqlang"),
    },
    outputChannelName: "reqlang",
  };

  lc = new LanguageClient(
    "reqlang-language-server",
    serverOptions,
    clientOptions
  );

  const startLanguageServerHandler = () => {
    return lc.start();
  };

  const stopLanguageServerHandler = () => {
    if (!lc) {
      return undefined;
    }

    return lc.stop();
  };

  const restartLanguageServerHandler = async () => {
    await stopLanguageServerHandler();

    await startLanguageServerHandler();
  };

  context.subscriptions.push(
    commands.registerCommand(
      "reqlang.startLanguageServer",
      startLanguageServerHandler
    ),
    commands.registerCommand(
      "reqlang.stopLanguageServer",
      stopLanguageServerHandler
    ),
    commands.registerCommand(
      "reqlang.restartLanguageServer",
      restartLanguageServerHandler
    ),
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

export function deactivate() {
  if (!lc) {
    return undefined;
  }

  return lc.stop();
}

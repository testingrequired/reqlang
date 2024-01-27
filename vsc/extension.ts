"use strict";

import {
  ExtensionContext,
  tasks,
  commands,
  env,
  Uri,
  workspace,
  window,
  StatusBarItem,
  StatusBarAlignment,
} from "vscode";
import { ReqlangTaskProvider } from "./reqlangTaskProvider";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";

let lc: LanguageClient;
let status: StatusBarItem;

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

  status = window.createStatusBarItem(StatusBarAlignment.Left, 0);
  status.command = "reqlang.setResolverEnv";
  status.text = "REQLANG";
  status.show();

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

  const installHandler = async () => {
    await stopLanguageServerHandler();

    const terminal = window.createTerminal(`reqlang`);
    terminal.show();
    terminal.sendText(`just install`, false);
    terminal.sendText("; exit");
  };

  const setResolverEnv = async () => {
    const env = await window.showInputBox({
      title: "Set the env for request file resolver to use",
      prompt: "Leave empty to clear the env",
    });

    if (env.length === 0) {
      return clearResolverEnv();
    }

    context.workspaceState.update("env", env);

    updateStatusText();
  };

  const clearResolverEnv = async () => {
    context.workspaceState.update("env", undefined);

    updateStatusText();
  };

  function updateStatusText() {
    const env = context.workspaceState.get("env");
    const text = typeof env === "undefined" ? "REQLANG" : `REQLANG(${env})`;

    status.text = text;
  }

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
    commands.registerCommand("reqlang.setResolverEnv", setResolverEnv),
    commands.registerCommand("reqlang.clearResolverEnv", clearResolverEnv),
    commands.registerCommand("reqlang.install", installHandler),
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

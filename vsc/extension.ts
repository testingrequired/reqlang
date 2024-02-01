"use strict";

import {
  ExtensionContext,
  Disposable,
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
let activeTextEditorHandler: Disposable;

type ReqfileState = {
  env: string;
};

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
    const env =
      (await window.showInputBox({
        title: "Set the env for request file resolver to use",
        prompt: "Leave empty to clear the env",
      })) ?? "";

    if (env.length === 0) {
      return clearResolverEnv();
    }

    const state: ReqfileState = {
      env,
    };

    if (!window.activeTextEditor) {
      return;
    }

    context.workspaceState.update(
      window.activeTextEditor.document.fileName,
      state
    );

    updateStatusText();
  };

  const clearResolverEnv = async () => {
    if (!window.activeTextEditor) {
      return;
    }

    context.workspaceState.update(
      window.activeTextEditor.document.fileName,
      undefined
    );

    updateStatusText();
  };

  function updateStatusText() {
    if (!window.activeTextEditor) {
      return;
    }

    const state: ReqfileState | undefined = context.workspaceState.get(
      window.activeTextEditor.document.fileName
    );

    const env = state?.env;
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

  function handleTextEditorChange() {
    if (!window.activeTextEditor) {
      return;
    }

    let filename = window.activeTextEditor.document.fileName;

    if (!filename.endsWith(".reqlang")) {
      return;
    }

    let state: ReqfileState;

    let existingState = context.workspaceState.get<ReqfileState>(
      window.activeTextEditor.document.fileName
    );

    if (typeof existingState === "undefined") {
      context.workspaceState.update(
        window.activeTextEditor.document.fileName,
        {}
      );

      state = context.workspaceState.get<ReqfileState>(
        window.activeTextEditor.document.fileName
      ) as ReqfileState;
    } else {
      state = existingState;
    }

    updateStatusText();
  }

  activeTextEditorHandler = window.onDidChangeActiveTextEditor(
    handleTextEditorChange
  );
}

export function deactivate() {
  activeTextEditorHandler?.dispose();

  if (!lc) {
    return undefined;
  }

  return lc.stop();
}

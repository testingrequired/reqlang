"use strict";

import {
  ExtensionContext,
  Disposable,
  commands,
  env,
  Uri,
  workspace,
  window,
  StatusBarItem,
  StatusBarAlignment,
} from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";

import * as RsResult from "rsresult";

import {
  Commands,
  ExecuteRequestParams,
  MenuChoices,
  type ParseNotification,
  type ParseResult,
  type ReqlangWorkspaceFileState,
} from "./src/types";

let client: LanguageClient;
let status: StatusBarItem;
let activeTextEditorHandler: Disposable;
let visibleTextEditorHandler: Disposable;

function initState(
  fileKey: string,
  context: ExtensionContext
): ReqlangWorkspaceFileState {
  const state = context.workspaceState.get<ReqlangWorkspaceFileState>(fileKey);

  if (typeof state === "undefined") {
    const initState: ReqlangWorkspaceFileState = {
      env: null,
      parseResult: null,
    };

    context.workspaceState.update(fileKey, initState);

    return initState;
  }

  return state;
}

function setEnv(
  fileKey: string,
  context: ExtensionContext,
  env: string | null
): ReqlangWorkspaceFileState {
  const state = initState(fileKey, context);

  state.env = env;

  context.workspaceState.update(fileKey, state);

  return state;
}

function getEnv(fileKey: string, context: ExtensionContext): string | null {
  const state = initState(fileKey, context);

  return state.env;
}

function getParseResults(
  fileKey: string,
  context: ExtensionContext
): RsResult.Result<ParseResult> | null {
  const state = initState(fileKey, context);

  return state.parseResult;
}

function setParseResult(
  fileKey: string,
  context: ExtensionContext,
  result: RsResult.Result<ParseResult>
): ReqlangWorkspaceFileState {
  const state = initState(fileKey, context);

  state.parseResult = result;

  context.workspaceState.update(fileKey, state);

  return state;
}

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

  client = new LanguageClient(
    "reqlang-language-server",
    serverOptions,
    clientOptions
  );

  const parseNotifications = client.onNotification(
    "reqlang/parse",
    async (params: ParseNotification) => {
      const state = setParseResult(params.file_id, context, params.result);

      client.outputChannel.appendLine(params.file_id);
      client.outputChannel.appendLine(
        JSON.stringify(state.parseResult, null, 2)
      );
      client.outputChannel.show();
    }
  );

  context.subscriptions.push(parseNotifications);

  status = window.createStatusBarItem(StatusBarAlignment.Left, 0);
  status.command = Commands.Menu;

  updateStatusText();

  const startLanguageServerHandler = () => {
    return client.start();
  };

  const stopLanguageServerHandler = () => {
    if (!client) {
      return undefined;
    }

    return client.stop();
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

  const exportToFile = async () => {
    const filename = await window.activeTextEditor?.document?.uri.toString()!;

    const state: ReqlangWorkspaceFileState | undefined =
      context.workspaceState.get(filename);

    let env = state?.env;

    if (!env) {
      env =
        (await window.showInputBox({
          title: "Set the env for request file resolver to use",
          prompt: "Leave empty to clear the env",
        })) ?? "";
    }

    const format =
      (await window.showInputBox({
        title: "Export to which format?",
        prompt: "Choose: http, curl, curl_script",
      })) ?? "http";

    const filename_to_save =
      (await window.showInputBox({
        title: "Path",
        prompt: "Enter a file path to save the curl script",
      })) ?? "curl_script.sh";

    const terminal = window.createTerminal(
      `reqlang export ${filename} as ${format} to ${filename_to_save}`
    );
    terminal.show();
    terminal.sendText(
      `reqlang ${filename} -f ${format} -e ${env} > ${filename_to_save}`
    );
  };

  const pickCurrentEnv = async () => {
    if (!window.activeTextEditor) {
      return;
    }

    let uri = window.activeTextEditor.document.uri.toString()!;

    let parseResult = getParseResults(uri, context);

    if (parseResult === null) {
      return;
    }

    await RsResult.ifOk(parseResult, async (parsedResult) => {
      if (!window.activeTextEditor) {
        return;
      }

      if (parsedResult.envs.length === 0) {
        return clearCurrentEnv();
      }

      const currentEnv = getEnv(uri, context);

      const env =
        (await window.showQuickPick(parsedResult.envs, {
          title: "Select environment for request",
          placeHolder: currentEnv ?? parsedResult.envs[0],
        })) ??
        currentEnv ??
        parsedResult.envs[0];

      if (env.length === 0) {
        return clearCurrentEnv();
      }

      setEnv(window.activeTextEditor.document.uri.toString(), context, env);

      updateStatusText();
    });
  };

  const clearCurrentEnv = async () => {
    if (!window.activeTextEditor) {
      return;
    }

    setEnv(window.activeTextEditor.document.uri.toString(), context, null);

    updateStatusText();
  };

  function updateStatusText() {
    if (!window.activeTextEditor) {
      status.hide();
      return;
    }

    const uri = window.activeTextEditor.document.uri.toString();

    if (!uri.endsWith(".reqlang")) {
      status.hide();
      return;
    }

    let parseResult = getParseResults(uri, context);

    if (parseResult === null) {
      client.outputChannel.appendLine("NULL");
      return;
    }

    RsResult.ifOkOr(
      parseResult,
      (parseResult) => {
        status.show();

        const state: ReqlangWorkspaceFileState | undefined =
          context.workspaceState.get(uri);

        const env = state?.env ?? "Select Environment";

        status.text = `http ${parseResult.request.verb} $(globe) ${env}`;
      },
      (_err) => {
        status.show();
        status.text = `http $(error) Error Parsing`;
      }
    );
  }

  context.subscriptions.push(
    commands.registerCommand(
      Commands.StartLanguageServer,
      startLanguageServerHandler
    ),
    commands.registerCommand(
      Commands.StopLanguageServer,
      stopLanguageServerHandler
    ),
    commands.registerCommand(
      Commands.RestartLanguageServer,
      restartLanguageServerHandler
    ),
    commands.registerCommand(Commands.Menu, async () => {
      if (!window.activeTextEditor) {
        return;
      }

      const uri = window.activeTextEditor.document.uri.toString();

      const choices: string[] = [];

      const env = getEnv(uri, context);

      if (!env) {
        choices.push(MenuChoices.PickEnv);
      } else {
        choices.push(MenuChoices.RunRequest);

        // Check if there is more than one environment
        const parseResult = getParseResults(uri, context);
        if (parseResult) {
          RsResult.ifOk(parseResult, (ok) => {
            if (ok.envs.length > 1) {
              choices.push(MenuChoices.PickEnv);
            }
          });
        }
      }

      const choice = await window.showQuickPick(choices, {
        title: "Reqlang Menu",
      });

      switch (choice) {
        case MenuChoices.PickEnv:
          await commands.executeCommand(Commands.PickEnv);
          break;

        case MenuChoices.ClearEnv:
          await commands.executeCommand(Commands.ClearEnv);
          break;

        case MenuChoices.RunRequest:
          await commands.executeCommand(Commands.RunRequest);
          break;

        default:
          break;
      }
    }),
    commands.registerCommand(Commands.PickEnv, pickCurrentEnv),
    commands.registerCommand(Commands.ClearEnv, clearCurrentEnv),
    commands.registerCommand(Commands.RunRequest, async () => {
      if (!window.activeTextEditor) {
        return;
      }

      let uri = window.activeTextEditor.document.uri.toString()!;

      let parseResult = getParseResults(uri, context);

      if (parseResult === null) {
        return;
      }

      await RsResult.ifOk(parseResult, async ({ prompts, secrets }) => {
        if (!window.activeTextEditor) {
          return;
        }

        const promptValues: (string | null)[] = [];
        const secretValues: (string | null)[] = [];
        const providerValues: (string | null)[] = [];

        for (const prompt of prompts) {
          const promptValue = await window.showInputBox({
            title: `Prompt: ${prompt}`,
          });

          promptValues.push(promptValue ?? null);
        }

        for (const secret of secrets) {
          const secretValue = await window.showInputBox({
            title: `Secret: ${secret}`,
          });

          secretValues.push(secretValue ?? null);
        }

        client.outputChannel.appendLine(
          JSON.stringify({
            prompts,
            promptValues,
            secrets,
            secretValues,
            providerValues,
          })
        );

        const uri = window.activeTextEditor.document.uri.toString()!;
        const env = getEnv(uri, context)!;
        const vars: Record<string, string> = {};

        const promptsObj: Record<string, string> = {};

        for (let i = 0; i < prompts.length; i++) {
          const key = prompts[i];
          const value = promptValues[i]!;

          promptsObj[key] = value;
        }

        const secretsObj: Record<string, string> = {};

        for (let i = 0; i < secrets.length; i++) {
          const key = secrets[i];
          const value = secretValues[i]!;

          secretsObj[key] = value;
        }

        const params: ExecuteRequestParams = {
          uri,
          env,
          vars,
          prompts: promptsObj,
          secrets: secretsObj,
        };

        const response = await commands.executeCommand<string>(
          Commands.Execute,
          params
        );

        client.outputChannel.appendLine(response);
      });
    }),
    commands.registerCommand(Commands.Install, installHandler),
    commands.registerCommand(Commands.OpenMdnDocsHttp, () => {
      env.openExternal(
        Uri.parse("https://developer.mozilla.org/en-US/docs/Web/HTTP")
      );
    }),
    commands.registerCommand(Commands.OpenMdnDocsHttpMessages, () => {
      env.openExternal(
        Uri.parse("https://developer.mozilla.org/en-US/docs/Web/HTTP/Messages")
      );
    }),
    commands.registerCommand(Commands.OpenMdnDocsHttpSpecs, () => {
      env.openExternal(
        Uri.parse(
          "https://developer.mozilla.org/en-US/docs/Web/HTTP/Resources_and_specifications"
        )
      );
    }),
    commands.registerCommand(Commands.ExportToFile, exportToFile)
  );

  function handleTextEditorChange() {
    if (!window.activeTextEditor) {
      updateStatusText();
      return;
    }

    let filename = window.activeTextEditor.document.uri.toString();

    if (!filename.endsWith(".reqlang")) {
      updateStatusText();
      return;
    }

    initState(filename, context);

    // Default the selected environment is there's just one
    RsResult.ifOk(getParseResults(filename, context)!, (result) => {
      if (result.envs.length === 1) {
        setEnv(filename, context, result.envs[0]);
      }
    });

    updateStatusText();
  }

  activeTextEditorHandler = window.onDidChangeActiveTextEditor(
    handleTextEditorChange
  );

  visibleTextEditorHandler = window.onDidChangeVisibleTextEditors(
    handleTextEditorChange
  );
}

export function deactivate() {
  activeTextEditorHandler?.dispose();
  visibleTextEditorHandler?.dispose();

  if (!client) {
    return undefined;
  }

  return client.stop();
}

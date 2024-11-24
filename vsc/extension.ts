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

import type { Request } from "reqlang-types";

type Ok<T> = { Ok: T };
type Err<E = unknown> = { Err: E };
type Result<T, E = unknown> = Ok<T> | Err<E>;

function isOk<T>(value: unknown): value is Ok<T> {
  const keys = Object.keys(value as object);

  return keys.includes("Ok") && keys.length === 1;
}

function isErr<E = unknown>(value: unknown): value is Err<E> {
  const keys = Object.keys(value as object);

  return keys.includes("Err") && keys.length === 1;
}

function ifOk<T, F extends (value: T) => void | Promise<void>>(
  value: Result<T>,
  fn: F
): ReturnType<F> | void {
  if (isOk(value)) {
    return fn(value.Ok) as ReturnType<F>;
  }

  return undefined as ReturnType<F>; // Explicitly cast `undefined` to match the type
}

function ifOkOr<
  T,
  F extends (value: T) => void | Promise<void>,
  E extends (value: unknown) => void | Promise<void>
>(value: Result<T>, okFn: F, errFn: E) {
  if (isOk(value)) {
    lc.outputChannel.appendLine("OK");
    return okFn(value.Ok);
  } else {
    lc.outputChannel.appendLine(`ERR: ${value.Err}`);
    return errFn(value.Err);
  }
}

function mapResult<T, U, E = unknown>(
  result: Result<T, E>,
  fn: (value: T) => U
): Result<U, E> {
  if (isOk(result)) {
    return { Ok: fn(result.Ok) };
  }
  return result;
}

let lc: LanguageClient;
let status: StatusBarItem;
let activeTextEditorHandler: Disposable;
let visibleTextEditorHandler: Disposable;

/**
 * State for an individual request file
 */
type ReqlangWorkspaceFileState = {
  /**
   * Current selected environment
   */
  env: string | null;
  parseResult: Result<ParseResult> | null;
};

type ParseNotification = {
  file_id: string;
  result: Result<ParseResult>;
};

type ParseResult = {
  vars: string[];
  envs: string[];
  prompts: string[];
  secrets: string[];
  request: Request;
};

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
): Result<ParseResult> | null {
  const state = initState(fileKey, context);

  return state.parseResult;
}

function setParseResult(
  fileKey: string,
  context: ExtensionContext,
  result: Result<ParseResult>
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

  lc = new LanguageClient(
    "reqlang-language-server",
    serverOptions,
    clientOptions
  );

  const parseNotifications = lc.onNotification(
    "reqlang/parse",
    async (params: ParseNotification) => {
      const state = setParseResult(params.file_id, context, params.result);

      lc.outputChannel.appendLine(params.file_id);
      lc.outputChannel.appendLine(JSON.stringify(state.parseResult, null, 2));
      lc.outputChannel.show();
    }
  );

  context.subscriptions.push(parseNotifications);

  status = window.createStatusBarItem(StatusBarAlignment.Left, 0);
  status.command = "reqlang.setResolverEnv";

  updateStatusText();

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

  const setResolverEnv = async () => {
    if (!window.activeTextEditor) {
      return;
    }

    let uri = window.activeTextEditor.document.uri.toString()!;

    let parseResult = getParseResults(uri, context);

    if (parseResult === null) {
      return;
    }

    await ifOk(
      mapResult(parseResult, (parsed) => parsed.envs),
      async (envs) => {
        if (!window.activeTextEditor) {
          return;
        }

        if (envs.length === 0) {
          return clearResolverEnv();
        }

        const currentEnv = getEnv(uri, context);

        const env =
          (await window.showQuickPick(envs, {
            title: "Select environment for request",
            placeHolder: currentEnv ?? envs[0],
          })) ??
          currentEnv ??
          envs[0];

        if (env.length === 0) {
          return clearResolverEnv();
        }

        setEnv(window.activeTextEditor.document.uri.toString(), context, env);

        updateStatusText();
      }
    );
  };

  const clearResolverEnv = async () => {
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
      lc.outputChannel.appendLine("NULL");
      return;
    }

    ifOkOr(
      mapResult(parseResult, (x) => x.request),
      (request) => {
        status.show();

        const state: ReqlangWorkspaceFileState | undefined =
          context.workspaceState.get(uri);

        const env = state?.env ?? "Select Environment";

        status.text = `http ${request.verb} $(globe) ${env}`;
      },
      () => {
        status.show();
        status.text = `http $(error) Error Parsing`;
      }
    );
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
    }),
    commands.registerCommand("reqlang.exportToFile", exportToFile)
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

    initState(window.activeTextEditor.document.uri.toString(), context);

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

  if (!lc) {
    return undefined;
  }

  return lc.stop();
}

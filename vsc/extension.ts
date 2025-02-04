"use strict";

import {
  ExtensionContext,
  Disposable,
  commands,
  window,
  languages,
} from "vscode";

import { type ParseNotificationFromServer } from "./src/types";
import * as state from "./src/state";
import { getClient } from "./src/client";
import {
  clearCurrentEnv,
  Commands,
  exportToFile,
  menuHandler,
  openMdnHttpDocs,
  openMdnHttpDocsMessages,
  openMdnHttpDocsSpecs,
  pickCurrentEnv,
  restartLanguageServer,
  runRequest,
  showResponse,
  startLanguageServer,
  stopLanguageServer,
} from "./src/commands";
import { ReqlangCodeLensProvider } from "./src/codelens";

let activeTextEditorHandler: Disposable;
let visibleTextEditorHandler: Disposable;

export async function activate(context: ExtensionContext) {
  await startLanguageServer();

  context.subscriptions.push(
    commands.registerCommand(Commands.StartLanguageServer, startLanguageServer),
    commands.registerCommand(Commands.StopLanguageServer, stopLanguageServer),
    commands.registerCommand(
      Commands.RestartLanguageServer,
      restartLanguageServer,
    ),
    commands.registerCommand(Commands.Menu, menuHandler(context)),
    commands.registerCommand(Commands.PickEnv, pickCurrentEnv(context)),
    commands.registerCommand(Commands.ClearEnv, (context) =>
      clearCurrentEnv(context),
    ),
    commands.registerCommand(Commands.RunRequest, runRequest(context)),
    commands.registerCommand(Commands.OpenMdnDocsHttp, openMdnHttpDocs),
    commands.registerCommand(
      Commands.OpenMdnDocsHttpMessages,
      openMdnHttpDocsMessages,
    ),
    commands.registerCommand(
      Commands.OpenMdnDocsHttpSpecs,
      openMdnHttpDocsSpecs,
    ),
    commands.registerCommand(Commands.ExportToFile, exportToFile(context)),
    commands.registerCommand(Commands.ShowResponse, showResponse),
    commands.registerCommand(
      Commands.DebugResetWorkspaceState,
      state.debugResetCurrentFileState(context),
    ),
  );

  context.subscriptions.push(
    languages.registerCodeLensProvider(
      "reqlang",
      new ReqlangCodeLensProvider(context),
    ),
  );

  state.initCurrentFileState(context);

  activeTextEditorHandler = window.onDidChangeActiveTextEditor(() =>
    state.initCurrentFileState(context),
  );

  visibleTextEditorHandler = window.onDidChangeVisibleTextEditors(() =>
    state.initCurrentFileState(context),
  );

  subscribeToParseNotificationsFromServer(context);
}

export function deactivate() {
  activeTextEditorHandler?.dispose();
  visibleTextEditorHandler?.dispose();

  stopLanguageServer();
}

/**
 * Subscribes to parse notifications from the language server.
 *
 * These happen on reqfile open and saves.
 *
 * @param context The extension context
 */
function subscribeToParseNotificationsFromServer(context: ExtensionContext) {
  context.subscriptions.push(
    getClient().onNotification(
      "reqlang/parse",
      async (params: ParseNotificationFromServer) => {
        getClient().outputChannel.appendLine(
          `Recieved parse notification from language server for '${params.file_id}': ${JSON.stringify(params.result)}\n`,
        );

        state.setParseResult(params.file_id, context, params.result);
      },
    ),
  );
}

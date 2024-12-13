"use strict";

import {
  ExtensionContext,
  Disposable,
  commands,
  window,
  languages,
} from "vscode";

import * as RsResult from "rsresult";

import { Commands, type ParseNotification } from "./src/types";
import * as state from "./src/state";
import { getClient } from "./src/client";
import {
  clearCurrentEnv,
  menuHandler,
  openMdnHttpDocs,
  openMdnHttpDocsMessages,
  openMdnHttpDocsSpecs,
  pickCurrentEnv,
  restartLanguageServer,
  runRequest,
  startLanguageServer,
  stopLanguageServer,
} from "./src/commands";
import * as statusBar from "./src/status";
import { ReqlangCodeLensProvider } from "./src/codelens";

let activeTextEditorHandler: Disposable;
let visibleTextEditorHandler: Disposable;

export function activate(context: ExtensionContext) {
  // Initialize and update the status bar
  const updateStatusText = statusBar.updateStatusText(context);
  updateStatusText();

  context.subscriptions.push(
    commands.registerCommand(Commands.StartLanguageServer, startLanguageServer),
    commands.registerCommand(Commands.StopLanguageServer, stopLanguageServer),
    commands.registerCommand(
      Commands.RestartLanguageServer,
      restartLanguageServer
    ),
    commands.registerCommand(Commands.Menu, menuHandler(context)),
    commands.registerCommand(Commands.PickEnv, pickCurrentEnv(context)),
    commands.registerCommand(Commands.ClearEnv, clearCurrentEnv(context)),
    commands.registerCommand(Commands.RunRequest, runRequest(context)),
    commands.registerCommand(Commands.OpenMdnDocsHttp, openMdnHttpDocs),
    commands.registerCommand(
      Commands.OpenMdnDocsHttpMessages,
      openMdnHttpDocsMessages
    ),
    commands.registerCommand(
      Commands.OpenMdnDocsHttpSpecs,
      openMdnHttpDocsSpecs
    )
  );

  context.subscriptions.push(
    languages.registerCodeLensProvider(
      "reqlang",
      new ReqlangCodeLensProvider(context)
    )
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

    state.initState(filename, context);

    // Default the selected environment is there's just one
    RsResult.ifOk(state.getParseResults(filename, context)!, (result) => {
      if (result.envs.length === 1) {
        state.setEnv(filename, context, result.envs[0]);
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
      async (params: ParseNotification) => {
        const newState = state.setParseResult(
          params.file_id,
          context,
          params.result
        );

        getClient().outputChannel.appendLine(params.file_id);
        getClient().outputChannel.appendLine(
          JSON.stringify(newState.parsedReqfile, null, 2)
        );
        getClient().outputChannel.show();
      }
    )
  );
}

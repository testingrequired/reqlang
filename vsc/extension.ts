"use strict";

import { ExtensionContext, Disposable, commands, window } from "vscode";

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
  restartLanguageServerHandler,
  runRequest,
  startLanguageServerHandler,
  stopLanguageServerHandler,
} from "./src/commands";
import * as statusBar from "./src/status";

let activeTextEditorHandler: Disposable;
let visibleTextEditorHandler: Disposable;

export function activate(context: ExtensionContext) {
  const client = getClient();

  const parseNotifications = client.onNotification(
    "reqlang/parse",
    async (params: ParseNotification) => {
      const newState = state.setParseResult(
        params.file_id,
        context,
        params.result
      );

      client.outputChannel.appendLine(params.file_id);
      client.outputChannel.appendLine(
        JSON.stringify(newState.parsedReqfile, null, 2)
      );
      client.outputChannel.show();
    }
  );

  context.subscriptions.push(parseNotifications);

  // Initialize and update the status bar
  const updateStatusText = statusBar.updateStatusText(context);
  updateStatusText();

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
}

export function deactivate() {
  activeTextEditorHandler?.dispose();
  visibleTextEditorHandler?.dispose();

  stopLanguageServerHandler();
}

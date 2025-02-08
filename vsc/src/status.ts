import {
  ExtensionContext,
  StatusBarAlignment,
  StatusBarItem,
  window,
} from "vscode";
import { ReqfileState } from "./types";
import { getParseResults } from "./state";
import { getClient } from "./client";
import * as RsResult from "rsresult";
import { Commands } from "./commands";

let status: StatusBarItem;

export function getStatus(): StatusBarItem {
  initStatus();
  return status;
}

export function initStatus() {
  if (!status) {
    status = window.createStatusBarItem(StatusBarAlignment.Left, 0);
    status.command = Commands.Menu;
  }
}

export const updateStatusText = (context: ExtensionContext) => {
  const status = getStatus();

  if (!window.activeTextEditor) {
    status.hide();
    return;
  }

  const uri = window.activeTextEditor.document.uri.toString();

  if (!uri.endsWith(".reqlang")) {
    status.hide();
    return;
  }

  const parseResult = getParseResults(uri, context);

  if (parseResult === null) {
    const client = getClient();

    client.outputChannel.appendLine("NULL");
    return;
  }

  status.show();

  RsResult.ifOkOr(
    parseResult,
    (parseResult) => {
      const state = context.workspaceState.get<ReqfileState>(uri);

      const http = `HTTP/${parseResult.request.http_version} ${parseResult.request.verb}`;
      const env =
        parseResult.envs.length > 0
          ? ` $(globe) ${state?.env ?? "Select Environment"}`
          : "";

      status.text = `${http}${env}`;
    },
    (_err) => {
      status.text = `http $(error) Error Parsing`;
    },
  );
};

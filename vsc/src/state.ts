"use strict";
import { ExtensionContext, window } from "vscode";
import type {
  ReqlangWorkspaceFileState,
  SimplifiedParsedRequestFile,
} from "./types";
import * as RsResult from "rsresult";
import * as statusBar from "./status";
import { getClient } from "./client";

export const updateStatusText = (context: ExtensionContext) => () => {
  const status = statusBar.getStatus();

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
    const client = getClient();

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
};

export function setParseResult(
  fileKey: string,
  context: ExtensionContext,
  result: RsResult.Result<SimplifiedParsedRequestFile>
): ReqlangWorkspaceFileState {
  const state = initState(fileKey, context);

  state.parsedReqfile = result;

  context.workspaceState.update(fileKey, state);

  return state;
}

export function getParseResults(
  fileKey: string,
  context: ExtensionContext
): RsResult.Result<SimplifiedParsedRequestFile> | null {
  const state = initState(fileKey, context);

  return state.parsedReqfile;
}

export function initState(
  fileKey: string,
  context: ExtensionContext
): ReqlangWorkspaceFileState {
  const state = context.workspaceState.get<ReqlangWorkspaceFileState>(fileKey);

  if (typeof state === "undefined") {
    const initState: ReqlangWorkspaceFileState = {
      env: null,
      parsedReqfile: null,
    };

    context.workspaceState.update(fileKey, initState);

    return initState;
  }

  return state;
}
export function setEnv(
  fileKey: string,
  context: ExtensionContext,
  env: string | null
): ReqlangWorkspaceFileState {
  const state = initState(fileKey, context);

  state.env = env;

  context.workspaceState.update(fileKey, state);

  return state;
}
export function getEnv(
  fileKey: string,
  context: ExtensionContext
): string | null {
  const state = initState(fileKey, context);

  return state.env;
}

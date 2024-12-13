import {
  commands,
  env,
  ExtensionContext,
  Uri,
  window,
  workspace,
} from "vscode";
import { getClient, getClientWithoutInit } from "./client";
import * as state from "./state";
import { Commands, ExecuteRequestParams, MenuChoices } from "./types";
import * as RsResult from "rsresult";

export const startLanguageServerHandler = () => {
  const client = getClient();
  return client.start();
};

export const stopLanguageServerHandler = () => {
  const client = getClientWithoutInit();

  if (!client) {
    return undefined;
  }

  return client.stop();
};

export const pickCurrentEnv = (context: ExtensionContext) => async () => {
  if (!window.activeTextEditor) {
    return;
  }

  let uri = window.activeTextEditor.document.uri.toString()!;

  let parseResult = state.getParseResults(uri, context);

  if (parseResult === null) {
    return;
  }

  await RsResult.ifOk(parseResult, async (parsedResult) => {
    if (!window.activeTextEditor) {
      return;
    }

    if (parsedResult.envs.length === 0) {
      return clearCurrentEnv(context)();
    }

    const currentEnv = state.getEnv(uri, context);

    const env =
      (await window.showQuickPick(parsedResult.envs, {
        title: "Select environment for request",
        placeHolder: currentEnv ?? parsedResult.envs[0],
      })) ??
      currentEnv ??
      parsedResult.envs[0];

    if (env.length === 0) {
      return clearCurrentEnv(context)();
    }

    state.setEnv(window.activeTextEditor.document.uri.toString(), context, env);

    state.updateStatusText(context)();
  });
};

export const clearCurrentEnv = (context: ExtensionContext) => async () => {
  if (!window.activeTextEditor) {
    return;
  }

  state.setEnv(window.activeTextEditor.document.uri.toString(), context, null);

  state.updateStatusText(context)();
};

export const restartLanguageServerHandler = async () => {
  await stopLanguageServerHandler();

  await startLanguageServerHandler();
};

export const menuHandler = (context: ExtensionContext) => async () => {
  if (!window.activeTextEditor) {
    return;
  }

  const uri = window.activeTextEditor.document.uri.toString();

  const choices: string[] = [];

  const env = state.getEnv(uri, context);

  if (!env) {
    choices.push(MenuChoices.PickEnv);
  } else {
    choices.push(MenuChoices.RunRequest);

    // Check if there is more than one environment
    const parseResult = state.getParseResults(uri, context);
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
};

export const openMdnHttpDocs = () => {
  env.openExternal(
    Uri.parse("https://developer.mozilla.org/en-US/docs/Web/HTTP")
  );
};

export const openMdnHttpDocsMessages = () => {
  env.openExternal(
    Uri.parse("https://developer.mozilla.org/en-US/docs/Web/HTTP/Messages")
  );
};

export const openMdnHttpDocsSpecs = () => {
  env.openExternal(
    Uri.parse(
      "https://developer.mozilla.org/en-US/docs/Web/HTTP/Resources_and_specifications"
    )
  );
};

export const runRequest = (context: ExtensionContext) => async () => {
  if (!window.activeTextEditor) {
    return;
  }

  let uri = window.activeTextEditor.document.uri.toString()!;

  let parseResult = state.getParseResults(uri, context);

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

    const client = getClient();

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
    const env = state.getEnv(uri, context)!;
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

    // Put response string in to a new file in the workspace
    // Create a new untitled document
    const document = await workspace.openTextDocument({
      content: response, // Initial content for the document
      language: "json", // Specify the language mode, e.g., 'plaintext', 'javascript', etc.
    });

    // Show the document in the editor
    await window.showTextDocument(document);

    // Format the response json
    await commands.executeCommand("editor.action.formatDocument");
  });
};

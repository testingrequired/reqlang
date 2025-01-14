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
import { ExportRequestParams, MenuChoices } from "./types";
import * as RsResult from "rsresult";
import { updateStatusText } from "./status";
import { HttpResponse, RequestParamsFromClient } from "reqlang-types";

export enum Commands {
  PickEnv = "reqlang.pickEnv",
  ClearEnv = "reqlang.clearEnv",
  /**
   * Show UI to input prompts and secrets to run a request
   */
  RunRequest = "reqlang.run",
  Menu = "reqlang.menu",
  /**
   * Command executed on the language server
   */
  Execute = "reqlang.executeRequest",
  Export = "reqlang.exportRequest",
  StartLanguageServer = "reqlang.startLanguageServer",
  StopLanguageServer = "reqlang.stopLanguageServer",
  RestartLanguageServer = "reqlang.restartLanguageServer",
  Install = "reqlang.install",
  OpenMdnDocsHttp = "reqlang.openMdnDocsHttp",
  OpenMdnDocsHttpMessages = "reqlang.openMdnDocsHttpMessages",
  OpenMdnDocsHttpSpecs = "reqlang.openMdnDocsHttpSpecs",
  ExportToFile = "reqlang.exportToFile",
  DebugResetWorkspaceState = "reqlang.debugResetWorkspaceState",
  ShowResponse = "reqlang.showResponse",
}

export const startLanguageServer = () => {
  const client = getClient();
  return client.start();
};

export const stopLanguageServer = () => {
  const client = getClientWithoutInit();

  return client?.stop();
};

export const pickCurrentEnv = (context: ExtensionContext) => async () => {
  if (!window.activeTextEditor) {
    return;
  }

  const uri = window.activeTextEditor.document.uri.toString()!;

  const parseResult = state.getParseResults(uri, context);

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

    updateStatusText(context);
  });
};

export const clearCurrentEnv = (context: ExtensionContext) => async () => {
  if (!window.activeTextEditor) {
    return;
  }

  state.setEnv(window.activeTextEditor.document.uri.toString(), context, null);

  updateStatusText(context);
};

export const restartLanguageServer = async () => {
  await stopLanguageServer();

  await startLanguageServer();
};

export const menuHandler = (context: ExtensionContext) => async () => {
  if (!window.activeTextEditor) {
    return;
  }

  const uri = window.activeTextEditor.document.uri.toString();

  const choices: string[] = [];

  const env = state.getEnv(uri, context);

  // Check if there is more than one environment
  const parseResult = state.getParseResults(uri, context);
  if (parseResult) {
    RsResult.ifOk(parseResult, (ok) => {
      if (ok.envs.length > 1) {
        choices.push(MenuChoices.PickEnv);
        choices.push(MenuChoices.ClearEnv);
      }
    });
  }

  if (env) {
    choices.push(MenuChoices.RunRequest);
    choices.push(MenuChoices.ExportRequest);
  }

  const client = getClient();

  if (client.isRunning()) {
    choices.push(MenuChoices.RestartLanguageServer);
    choices.push(MenuChoices.StopLanguageServer);
  } else {
    choices.push(MenuChoices.StartLanguageServer);
  }

  choices.push(MenuChoices.OpenOutput, MenuChoices.DebugClearWorkspaceState);

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

    case MenuChoices.ExportRequest:
      await commands.executeCommand(Commands.ExportToFile);
      break;

    case MenuChoices.OpenOutput:
      client.outputChannel.show();
      break;

    case MenuChoices.DebugClearWorkspaceState:
      await commands.executeCommand(Commands.DebugResetWorkspaceState);
      break;

    default:
      break;
  }
};

export const showResponse = async (response: HttpResponse | null) => {
  if (response === null) {
    return;
  }

  const statusCode = response.status_code;
  const hasBody = response.body !== null && response.body !== "";

  if (statusCode >= 200 && statusCode <= 299) {
    const actions = ["Show Response"];

    if (hasBody) {
      actions.push("Show Body");
    }

    actions.push("Ok");

    window
      .showInformationMessage(
        `Success! Status: ${response.status_code}`,
        ...actions,
      )
      .then(async (action) => {
        switch (action) {
          case "Show Response": {
            // Create a new untitled document // Put response string in to a new file in the workspace
            const responseDocument = await workspace.openTextDocument({
              content: JSON.stringify(response, null, 2), // Initial content for the document
              language: "json", // Specify the language mode, e.g., 'plaintext', 'javascript', etc.
            });

            // Show the document in the editor
            await window.showTextDocument(responseDocument);
            break;
          }
          case "Show Body": {
            // Try to get content type of the response
            const contentType =
              response.headers["content-type"] ??
              response.headers["Content-Type"];

            // Try to determine the language of the response based on the
            // content type
            let language: string;
            switch (true) {
              case contentType?.startsWith("application/json"):
                language = "json";
                break;
              case contentType?.startsWith("text/html"):
                language = "html";
                break;
              default:
                language = "plaintext";
            }

            // Put response string in to a new file in the workspace
            // Create a new untitled document
            const document = await workspace.openTextDocument({
              content: response.body ?? "", // Initial content for the document
              language, // Specify the language mode, e.g., 'plaintext', 'javascript', etc.
            });

            // Show the document in the editor
            await window.showTextDocument(document);
            break;
          }
          default:
            break;
        }
      });
  } else {
    const action = await window.showErrorMessage(
      `Failed! Status: ${response.status_code}`,
      "Show Response",
      "Ok",
    );

    switch (action) {
      case "Show Response": {
        // Create a new untitled document // Put response string in to a new file in the workspace
        const document = await workspace.openTextDocument({
          content: JSON.stringify(response, null, 2), // Initial content for the document
          language: "json", // Specify the language mode, e.g., 'plaintext', 'javascript', etc.
        });

        // Show the document in the editor
        await window.showTextDocument(document);

        // Format the response json
        await commands.executeCommand("editor.action.formatDocument");
        break;
      }
      case "Ok":
        break;
    }
  }
};

export const openMdnHttpDocs = () => {
  env.openExternal(
    Uri.parse("https://developer.mozilla.org/en-US/docs/Web/HTTP"),
  );
};

export const openMdnHttpDocsMessages = () => {
  env.openExternal(
    Uri.parse("https://developer.mozilla.org/en-US/docs/Web/HTTP/Messages"),
  );
};

export const openMdnHttpDocsSpecs = () => {
  env.openExternal(
    Uri.parse(
      "https://developer.mozilla.org/en-US/docs/Web/HTTP/Resources_and_specifications",
    ),
  );
};

/**
 * Create command handler for running requests
 * @param context Extension context for VS Code
 * @returns Command handler for running requests
 */
export const runRequest =
  (context: ExtensionContext) =>
  /**
   * Handle command to run requests
   */
  async () => {
    if (!window.activeTextEditor) {
      return;
    }

    const uri = window.activeTextEditor.document.uri.toString()!;

    const lastResponse = state.getLastResponse(uri, context);

    const parseResult = state.getParseResults(uri, context);

    if (parseResult === null) {
      return;
    }

    if (state.getIsWaitingForResponse(uri, context)) {
      return;
    }

    await RsResult.ifOk(parseResult, async ({ prompts, secrets }) => {
      if (!window.activeTextEditor) {
        return;
      }

      const promptValues: (string | null)[] = [];
      const secretValues: (string | null)[] = [];

      for (const prompt of prompts) {
        const promptValue = await window.showInputBox({
          title: `Prompt: ${prompt}`,
          value: lastResponse?.params.prompts[prompt],
        });

        if (promptValue === undefined) {
          return;
        }

        promptValues.push(promptValue);
      }

      for (const secret of secrets) {
        const secretValue = await window.showInputBox({
          title: `Secret: ${secret}`,
        });

        if (secretValue === undefined) {
          return;
        }

        secretValues.push(secretValue);
      }

      const vars: Record<string, string> = {};

      // Prompts

      const promptsObj: Record<string, string> = {};

      for (let i = 0; i < prompts.length; i++) {
        const key = prompts[i];
        const value = promptValues[i]!;

        promptsObj[key] = value;
      }

      // Secrets

      const secretsObj: Record<string, string> = {};

      for (let i = 0; i < secrets.length; i++) {
        const key = secrets[i];
        const value = secretValues[i]!;

        secretsObj[key] = value;
      }

      const requestStartDate = new Date();

      // Set state to know the request has been sent to the language server
      // It's used to set UI state in the editor
      state.setIsWaitingForResponse(uri, context, true);

      const reqfile_text = window.activeTextEditor.document.getText();

      const env = state.getEnv(uri, context)!;

      const requestParamsToServer: RequestParamsFromClient = {
        reqfile: reqfile_text,
        env,
        vars,
        prompts: promptsObj,
        secrets: secretsObj,
      };

      /**
       * HTTP Response from language server
       */
      const responseJson = await commands.executeCommand<string>(
        Commands.Execute,
        requestParamsToServer,
      );

      const response: HttpResponse = JSON.parse(responseJson);
      const statusCode = response.status_code;

      state.setLastResponse(uri, context, {
        startDateIso: requestStartDate.toISOString(),
        response,
        endDateIso: new Date().toISOString(),
        wasSuccessful: statusCode >= 200 && statusCode < 300,
        params: requestParamsToServer,
      });

      // Set state to know the request has been received
      state.setIsWaitingForResponse(uri, context, false);

      commands.executeCommand(Commands.ShowResponse, response);
    });
  };

export const exportToFile = (context: ExtensionContext) => async () => {
  if (!window.activeTextEditor) {
    return;
  }

  const uri = window.activeTextEditor.document.uri.toString()!;

  const parseResult = state.getParseResults(uri, context);

  if (parseResult === null) {
    return;
  }

  if (state.getIsWaitingForResponse(uri, context)) {
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

      if (promptValue === undefined) {
        return;
      }

      promptValues.push(promptValue ?? null);
    }

    for (const secret of secrets) {
      const secretValue = await window.showInputBox({
        title: `Secret: ${secret}`,
      });

      if (secretValue === undefined) {
        return;
      }

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
      }),
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

    const params: ExportRequestParams = {
      uri,
      env,
      vars,
      prompts: promptsObj,
      secrets: secretsObj,
      format: "CurlScript",
    };

    const response = await commands.executeCommand<string>(
      Commands.Export,
      params,
    );

    // Put response string in to a new file in the workspace
    // Create a new untitled document
    const document = await workspace.openTextDocument({
      content: response, // Initial content for the document
      language: "shellscript", // Specify the language mode, e.g., 'plaintext', 'javascript', etc.
    });

    // Show the document in the editor
    await window.showTextDocument(document);
  });
};

import { workspace } from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

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

/**
 * Initialize the LanguageClient instance if not already initialized.
 */
function initClient() {
  if (typeof client === "undefined") {
    client = new LanguageClient(
      "reqlang-language-server",
      serverOptions,
      clientOptions,
    );
  }
}

/**
 * Get or initialize the LanguageClient instance.
 *
 * @returns LanguageClient instance
 */
export function getClient(): LanguageClient {
  initClient();

  return client!;
}

/**
 * Get the LanguageClient instance without initializing it.
 *
 * @returns LanguageClient, if it's initialized. Otherwise, undefined.
 */
export function getClientWithoutInit(): LanguageClient | undefined {
  return client;
}

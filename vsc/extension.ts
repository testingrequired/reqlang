"use strict";

import { ExtensionContext, tasks } from "vscode";
import { ReqlangTaskProvider } from "./reqlangTaskProvider";

export function activate(context: ExtensionContext) {
  context.subscriptions.push();

  tasks.registerTaskProvider("reqlang", new ReqlangTaskProvider());
}

export function deactivate() {}

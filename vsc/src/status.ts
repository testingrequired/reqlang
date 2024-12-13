import { StatusBarAlignment, StatusBarItem, window } from "vscode";
import { Commands } from "./types";

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

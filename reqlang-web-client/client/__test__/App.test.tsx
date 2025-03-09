import React from "react";
import { expect, test } from "vitest";
import { render, screen, createEvent, fireEvent } from "@testing-library/react";
import App from "../src/App";
import { JSDOM } from "jsdom";

import "@testing-library/jest-dom/vitest";

test("displays message to upload a file if a file hasn't been uploaded", () => {
  render(<App />);

  expect(screen.getByText("reqlang-web-client")).toBeVisible();

  expect(screen.getByText("Drag & drop a request file")).toBeVisible();
});

test.skip("displays request file content after drag and drop", async () => {
  render(<App />);

  const dropzone = screen.getByTestId("uploader");

  const event = createEvent.drop(dropzone);

  const { File, FileList } = new JSDOM().window;

  const fileList = [new File(["Hello!"], "hello.reqlang")];

  // @ts-ignore
  fileList.__proto__ = Object.create(FileList.prototype);

  const reducer = (dataTransfer, file) => {
    dataTransfer.items.add(file);
    return dataTransfer;
  };

  const dt = fileList.reduce(reducer, new DataTransfer()).files;

  Object.defineProperty(event, "dataTransfer", {
    value: dt,
  });

  fireEvent(dropzone, event);

  expect(await screen.findByText("reqlang-web-client")).toBeVisible();

  expect(screen.getByText("Drag & drop a request file")).toBeVisible();

  expect(screen.getByText("Hello!")).toBeVisible();
});

import { expect, test } from "vitest";
import * as reqlang from "@testingrequired/reqlang-wasm";

const reqfile = `
#!/usr/bin/env reqlang
---
GET /posts/{{?id}} HTTP/1.1
host: {{:base_url}}
x-api-key: {{!api_key}}

---
HTTP/1.1 200 OK

{
  "id": "{{?id}}"
}
---
vars = ["base_url"]
secrets = ["api_key"]

[envs]
[envs.dev]
base_url = "http://example.com"

[prompts]
id = ""

---

`;

test("resolve should return json", () => {
  expect(
    reqlang.resolve(
      reqfile,
      "dev",
      {
        id: "test id value",
      },
      {
        api_key: "api key value",
      }
    )
  ).to.deep.equals({
    request: [
      {
        verb: "GET",
        target: "/posts/{{?id}}",
        http_version: "1.1",
        headers: new Map([
          ["host", "{{:base_url}}"],
          ["x-api-key", "{{!api_key}}"],
        ]),
        body: "",
      },
      {
        start: 28,
        end: 101,
      },
    ],
    response: [
      {
        http_version: "1.1",
        status_code: "200",
        status_text: "OK",
        headers: new Map(),
        body:
          JSON.stringify(
            {
              id: "{{?id}}",
            },
            null,
            2
          ) + "\n",
      },
      {
        start: 105,
        end: 144,
      },
    ],
    config: [
      {
        env: "dev",
        prompts: new Map([["id", "test id value"]]),
        secrets: new Map([["api_key", "api key value"]]),
        vars: new Map([["base_url", "http://example.com"]]),
      },
      { start: 148, end: 261 },
    ],
    refs: [
      [
        {
          Prompt: "id",
        },
        { start: 28, end: 101 },
      ],
      [{ Variable: "base_url" }, { start: 28, end: 101 }],
      [{ Secret: "api_key" }, { start: 28, end: 101 }],
      [{ Prompt: "id" }, { start: 105, end: 144 }],
    ],
  });
});

test("parse should return json", () => {
  expect(reqlang.parse(reqfile)).to.deep.equals({
    request: [
      {
        verb: "GET",
        target: "/posts/{{?id}}",
        http_version: "1.1",
        headers: new Map([
          ["host", "{{:base_url}}"],
          ["x-api-key", "{{!api_key}}"],
        ]),
        body: "",
      },
      {
        start: 28,
        end: 101,
      },
    ],
    response: [
      {
        http_version: "1.1",
        status_code: "200",
        status_text: "OK",
        headers: new Map(),
        body:
          JSON.stringify(
            {
              id: "{{?id}}",
            },
            null,
            2
          ) + "\n",
      },
      {
        start: 105,
        end: 144,
      },
    ],
    config: [
      {
        envs: new Map([["dev", new Map([["base_url", "http://example.com"]])]]),
        prompts: new Map([["id", ""]]),
        vars: undefined,
        secrets: ["api_key"],
        vars: ["base_url"],
      },
      { start: 148, end: 261 },
    ],
    refs: [
      [
        {
          Prompt: "id",
        },
        { start: 28, end: 101 },
      ],
      [{ Variable: "base_url" }, { start: 28, end: 101 }],
      [{ Secret: "api_key" }, { start: 28, end: 101 }],
      [{ Prompt: "id" }, { start: 105, end: 144 }],
    ],
  });
});

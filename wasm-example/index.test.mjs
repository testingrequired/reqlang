import { expect, test } from "vitest";
import * as reqlang from "@testingrequired/reqlang-wasm";

const reqfile = `
#!/usr/bin/env reqlang
---
GET /posts/{{?id}} HTTP/1.1
foo: bar
x-api-key: {{!api_key}}

---
HTTP/1.1 200 OK

{
  "id": "{{?id}}"
}
---
secrets = ["api_key"]

[envs]
[envs.dev]

[prompts]
id = ""

---

`;

const reqfile_single_header = `
#!/usr/bin/env reqlang
---
GET /posts/{{?id}} HTTP/1.1
x-api-key: {{!api_key}}

---
HTTP/1.1 200 OK

{
  "id": "{{?id}}"
}
---
secrets = ["api_key"]

[envs]
[envs.dev]

[prompts]
id = ""

---

`;

test("export should return formatted as Http", () => {
  expect(
    reqlang.export_to_format(
      reqfile,
      "dev",
      {
        id: "test_id_value",
      },
      {
        api_key: "api key value",
      },
      "Http"
    )
  ).to.deep.eq(
    `GET /posts/test_id_value HTTP/1.1\nfoo: bar\nx-api-key: api key value\n`
  );
});

test("export should return formatted as Curl", () => {
  expect(
    reqlang.export_to_format(
      reqfile_single_header,
      "dev",
      {
        id: "test_id_value",
      },
      {
        api_key: "api key value",
      },
      "Curl"
    )
  ).to.deep.eq(
    `curl /posts/test_id_value --http1.1 -H "x-api-key: api key value"`
  );
});

test("template should return json", () => {
  expect(
    reqlang.template(
      reqfile,
      "dev",
      {
        id: "test_id_value",
      },
      {
        api_key: "api key value",
      }
    )
  ).to.deep.equals({
    request: {
      verb: "GET",
      target: "/posts/test_id_value",
      http_version: "1.1",
      headers: new Map([
        ["foo", "bar"],
        ["x-api-key", "api key value"],
      ]),
      body: "",
    },
    response: {
      http_version: "1.1",
      status_code: "200",
      status_text: "OK",
      headers: new Map(),
      body:
        JSON.stringify(
          {
            id: "test_id_value",
          },
          null,
          2
        ) + "\n",
    },
  });
});

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
          ["x-api-key", "{{!api_key}}"],
          ["foo", "bar"],
        ]),
        body: "",
      },
      {
        start: 28,
        end: 90,
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
        start: 94,
        end: 133,
      },
    ],
    config: [
      {
        env: "dev",
        prompts: new Map([["id", "test id value"]]),
        secrets: new Map([["api_key", "api key value"]]),
        vars: new Map(),
      },
      { start: 137, end: 198 },
    ],
    refs: [
      [
        {
          Prompt: "id",
        },
        { start: 28, end: 90 },
      ],
      [{ Secret: "api_key" }, { start: 28, end: 90 }],
      [{ Prompt: "id" }, { start: 94, end: 133 }],
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
          ["foo", "bar"],
          ["x-api-key", "{{!api_key}}"],
        ]),
        body: "",
      },
      {
        start: 28,
        end: 90,
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
        start: 94,
        end: 133,
      },
    ],
    config: [
      {
        envs: new Map([["dev", new Map()]]),
        prompts: new Map([["id", ""]]),
        secrets: ["api_key"],
        vars: undefined,
      },
      { start: 137, end: 198 },
    ],
    refs: [
      [
        {
          Prompt: "id",
        },
        { start: 28, end: 90 },
      ],
      [{ Secret: "api_key" }, { start: 28, end: 90 }],
      [{ Prompt: "id" }, { start: 94, end: 133 }],
    ],
  });
});

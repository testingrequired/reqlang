import { expect, test } from "vitest";
import * as reqlang from "@testingrequired/reqlang-wasm";

const reqfile = `
---
GET / HTTP/1.1
host: {{:base_url}}

---
---
vars = ["base_url"]
---
`;

test("parse should return json", () => {
  expect(reqlang.parse(reqfile)).to.deep.equals({
    request: [
      {
        verb: "GET",
        target: "/",
        http_version: "1.1",
        headers: new Map([["host", "{{:base_url}}"]]),
        body: "",
      },
      {
        start: 5,
        end: 41,
      },
    ],
    response: undefined,
    config: [
      {
        envs: undefined,
        prompts: undefined,
        vars: undefined,
        secrets: undefined,
        vars: ["base_url"],
      },
      { start: 49, end: 69 },
    ],
    request_refs: [[{ Variable: "base_url" }, { start: 5, end: 41 }]],
    response_refs: [],
    config_refs: [],
  });
});

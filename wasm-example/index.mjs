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

console.log(JSON.stringify(reqlang.resolve(reqfile, "dev", {}, {}), null, 2));

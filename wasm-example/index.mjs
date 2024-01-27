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

console.log(JSON.stringify(reqlang.parse(reqfile), null, 2));

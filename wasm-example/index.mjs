import * as reqlang from "@testingrequired/reqlang-wasm";

const reqfile = `
---
GET / HTTP/1.1

---
`;

console.log(reqlang.parse(reqfile));

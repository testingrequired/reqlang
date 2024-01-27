import * as reqlang from "@testingrequired/reqlang-wasm";

const reqfile = `

`;

console.log(JSON.stringify(reqlang.parse(reqfile), null, 2));

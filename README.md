# Request Language

A file format specification for HTTP/s requests, response assertions, and associated data/configuration.

## Examples

See [all examples](./examples) for more.

### Request

Requests are written as HTTP request messages.

```reqlang
---
GET https://example.com HTTP/1.1
```

### Response Assertions

Responses are treated as an assertion and are written as an HTTP response message.

```reqlang
---
GET https://example.com HTTP/1.1
---
HTTP/1.1 200 OK
```

### Variables, Evironmental Values, & Template References

Requests and responses support templating be declaring variables and defining environment specific values.

```reqlang

vars = ["base_url"]

[envs.dev]
base_url = "https://dev.example.com"

[envs.prod]
base_url = "https://example.com"
---
GET {{:base_url}} HTTP/1.1
```

### Prompts

Prompts are input values to the request file and are supplied by the user.

```reqlang

[prompts]
example_id = ""
---
GET https://example.com/?id={{?example_id}} HTTP/1.1
```

### Secrets

Secrets are declared but their values are supplied at template time.

```reqlang

secrets = ["api_key"]
---
GET https://example.com HTTP/1.1
x-api-key: {{!api_key}}
```

## Usage

Reqlang's primary focus is being a file format specification but there are several tools available to generate and execute requests from these files.

## CLI

The [`reqlang`](./cli) CLI can parse request files as well export templated requests in to a variety of formats (`http`, `curl`, `curl_script`).

Adding `#!/usr/bin/env reqlang` to the top of request files allows easy usage of the CLI:

```reqlang
#!/usr/bin/env reqlang

[prompts]
# Status code the response will return
status_code = ""
---
GET https://httpbin.org/status/{{?status_code}} HTTP/1.1
```

Calling it:

```shell
./examples/valid/status_code.reqlang -e default -f curl -P status_code=201 | bash

HTTP/1.1 201 CREATED
Date: Sat, 14 Dec 2024 19:20:26 GMT
Content-Type: text/html; charset=utf-8
Content-Length: 0
Connection: keep-alive
Server: gunicorn/19.9.0
Access-Control-Allow-Origin: *
Access-Control-Allow-Credentials: true
```

## VS Code

The [VS Code extension](./vsc/#readme) acts as an in-editor REST client.

![VS Code Extension Screenshot](./vsc/screenshot.png)

## Desktop

The [desktop client](./reqlang-client) is a very simple GUI written in [egui](https://github.com/emilk/egui). It's mostly used for testing.

![GUI Client Screenshot](./reqlang-client/screenshot.png)

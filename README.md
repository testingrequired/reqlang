# Request Language

A file format specification for defining HTTP requests, response assertions, and configuration in "request files".

## Goals

- HTTP request and response messages
- Easy to read, write, and diff
- Lives in source control
- Environments
- Templating with variables, prompted, and secret values
- Client/implementation agnostic

### Future

- Chaining requests
- Response body mapping/transformation/extraction
- Authenticated requests (e.g. OAuth2) configuration
- Project workspaces

## Request Files

Request files (`*.reqlang`) are multi-document files containing a request along with an optional config and expected response. They are designed to define what the request is, not how to execute it (e.g. defining what secrets are needed instead of how to fetch them). This is left to implementing clients.

### Living Syntax

This is a living syntax subject to change wildly at anytime. The core concepts and goals will remain the same however.

### Example

[post.reqlang](./examples/valid/post.reqlang):

````reqlang
```%config
vars = ["test_value"]
secrets = ["super_secret_value"]

[[prompts]]
name = "prompt_value"

[envs.test]
test_value = "test_value"

[envs.prod]
test_value = "prod_value"

[envs.local]
test_value = "local_value"
```

```%request
POST https://httpbin.org/post HTTP/1.1

{
  "env": "{{@env}}",
  "value": "{{:test_value}}",
  "prompted_value": "{{?prompt_value}}",
  "secret_value": "{{!super_secret_value}}"
}
```
````

### Request

The request is the request is what's executed when the request file is ran. They are written as [HTTP request messages](https://developer.mozilla.org/en-US/docs/Web/HTTP/Messages#http_requests).

````reqlang
```%request
GET https://example.com HTTP/1.1
```
````

### Response

The response is optional but treated as an assertion if it is defined. When the request is executed, this response is compared to the actual response received.

````reqlang
```%request
GET https://example.com HTTP/1.1
```

```%response
HTTP/1.1 200 OK
```
````

#### Matching Rules

Client implementations can choose how to match the response against the expected response. Here are a list of recommended ways to match.

- Exact match `status code`
- Exact match `status text`
- Exact match `header value` of headers present in the expected response
- Exact match `body`
- Wildcard match `header value` and `body` using the `{{*}}` template references.

### Configuration

The configuration is optional but is used to define environment names and their variables as well as what prompts & secrets are needed. It currently uses the `toml` syntax.

#### Variables & Environments

Variables contain environmental variables that can be used in the request or response. A list of variable names is first declared.

Variables can be templated using the `{{:var_name}}` syntax. The environment of the request execution can be referenced using the `{{@env}}` syntax.

```toml
vars = ["user_id", "item_id"]
```

Then enviroments are declared with the appropriate values.

```toml
vars = ["user_id", "item_id"]

[envs.dev]
user_id = 12345
item_id = "abcd"

[envs.prod]
user_id = 67890
item_id = "efgh"
```

There is an implicitly defined `default` environment present but it still must be declared in the config.

```toml
vars = ["user_id"]

[envs.default]
user_id = 12345
```

##### Usage

````reqlang
```%config
vars = ["user_id", "item_id"]

[envs.dev]
user_id = 12345
item_id = "abcd"

[envs.prod]
user_id = 67890
item_id = "efgh"
```

```%request
GET https://{{@env}}.example.com/users/{{:user_id}}/items/{{:item_id}} HTTP/1.1
```
````

##### Goals

- Clearly define everything the request and response will need
- Declare environments once
- Require variable declaration before definition

###### Future

- Default value (implicitly set in the `default` environment)
- Value type

#### Prompts

Prompts are values provided by the user at request execution time. These are "inputs" to the request file. They can be templated in the request and responses using the `{{?prompt_name}}` syntax.

```toml
[[prompts]]
name = "tags"
description = "Tags included as a query param" # Optional
default = "tag1,tag2" # Optional
```

##### Usage

````reqlang
```%config
[[prompts]]
name = "tags"
```

```%request
GET https://example.com/posts?tags={{?tags}} HTTP/1.1
```
````

##### Future

- Default value
- Value type

#### Secrets

Secrets are protected values referenced by a name and declares what secrets will be required. How secret values are fetched is up to client implementations. They can be referenced using the `{{!secret_name}}` syntax.

```toml
secrets = ["api_key"]
```

##### Usage

````reqlang
```%config
secrets = ["api_key"]
```

```%request
GET https://example.com HTTP/1.1
x-api-key: {{!api_key}}
```
````

##### Goals

- Secret fetching is outside the scope of the request file

###### Future

- Configuring secret fetching in the workspace

### Examples

See [all examples](./examples) for more request files.

## Libraries

### Rust

The [reqlang](./reqlang/) crate is a library working with request files.

- [API Docs](https://testingrequired.github.io/reqlang/reqlang/)

```rust
use reqlang::prelude::*;

let request_file_text = fs::read_to_string("./path/to/requestfile.reqlang")
  .expect("Should have been able to read the file");

const ast = Ast::from(&request_file_text);

const parsed_request_file = parse(&ast).expect("should be a valid request file");
```

## Tooling

[![build-artifacts](https://github.com/testingrequired/reqlang/actions/workflows/build-artifacts.yml/badge.svg)](https://github.com/testingrequired/reqlang/actions/workflows/build-artifacts.yml)

These act as both tooling for request file and reference implementations for clients.

### CLI

The [`reqlang`](./cli) CLI validates and exports requests in to a variety of formats (`http`, `curl`, `json`).

```shell
reqlang
```

```
Command to work with request files

Usage: reqlang [COMMAND]

Commands:
  export  Export request to specified format
  ast     Produce an AST for a request file
  parse   Parse a request file
  run     Run a request file
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

#### Run

Execute the request from a request file.

```
Usage: reqlang run [OPTIONS] <path>

Arguments:
  <path>  Path to request file

Options:
  -e, --env <env>         Resolve with an environment
  -P, --prompt <prompts>  Input a prompt value
  -S, --secret <secrets>  Input a secret value
  -f, --format <format>   Format the response [default: http] [possible values: http, json, body]
  -t, --test              Test if the response matches the expected response, if defined
  -h, --help              Print help
```

##### Examples

```shell
reqlang run ./examples/valid/status_code.reqlang --prompt status_code=200
```

```
HTTP/1.1 200 OK
content-type: text/html; charset=utf-8
connection: keep-alive
content-length: 0
server: gunicorn/19.9.0
access-control-allow-credentials: true
access-control-allow-origin: *
```

##### Testing Responses

Run the response assertion, if defined in the request file, the response will be compared to the expected response.

```shell
reqlang run examples/valid/mismatch_response.reqlang --test
```

See: [mismatch_response.reqlang](./examples/valid/mismatch_response.reqlang)

```diff
HTTP/1.1 200 OK
connection: keep-alive
server: gunicorn/19.9.0
access-control-allow-origin: *
access-control-allow-credentials: true
date: Sun, 02 Feb 2025 03:55:33 GMT
content-type: application/json
content-length: 429

{
  "slideshow": {
    "author": "Yours Truly",
    "date": "date of publication",
    "slides": [
      {
        "title": "Wake up to WonderWidgets!",
        "type": "all"
      },
      {
        "items": [
          "Why <em>WonderWidgets</em> are great",
          "Who <em>buys</em> WonderWidgets"
        ],
        "title": "Overview",
        "type": "all"
      }
    ],
    "title": "Sample Slide Show"
  }
}

Response assertion failed:

-HTTP/1.1 201 Created
+HTTP/1.1 200 OK
-x-test-value: ...

 {
   "slideshow": {
-    "author": "Yours Truly",
+    "author": "Yours Truly",
+    "date": "date of publication",
     "slides": [
       {
         "title": "Wake up to WonderWidgets!",
         "type": "all"
       },
       {
         "items": [
           "Why <em>WonderWidgets</em> are great",
           "Who <em>buys</em> WonderWidgets"
         ],
         "title": "Overview",
         "type": "all"
       }
     ],
-    "title": "Test Slide Show"
-  },
-  "extra": true
+    "title": "Sample Slide Show"
+  }
 }
-
```

#### Parse

Validate and parse request files. It returns a JSON object with info about the request file: environment names, variables, prompts, secrets, the (untemplated) request itself.

```
Usage: reqlang parse <path>

Arguments:
  <path>  Path to request file

Options:
  -h, --help  Print help
```

##### Examples

```shell
reqlang parse ./examples/valid/status_code.reqlang
```

```json
{
  "vars": ["test_value"],
  "envs": ["prod", "test", "local"],
  "prompts": ["prompt_value"],
  "secrets": ["super_secret_value"],
  "request": {
    "verb": "POST",
    "target": "https://httpbin.org/post",
    "http_version": "1.1",
    "headers": [],
    "body": "{\n  \"env\": \"{{@env}}\",\n  \"value\": \"{{:test_value}}\",\n  \"prompted_value\": \"{{?prompt_value}}\",\n  \"secret_value\": \"{{!super_secret_value}}\"\n}\n\n"
  }
}
```

##### Filtering

Use tools like `jq` to extract specific information from the parsed request.

###### Environment Names

Let a list of environment names defined in the request file.

```shell
reqlang parse ./examples/valid/post.reqlang | jq '.envs'
```

```json
["local", "test", "prod"]
```

###### Variables

Let a list of variables provided by the request file.

```shell
reqlang parse ./examples/valid/post.reqlang | jq '.vars'
```

```json
["test_value"]
```

###### Prompts

Let a list of prompts required by the request file.

```shell
reqlang parse ./examples/valid/post.reqlang | jq '.prompts'
```

```json
["prompt_value"]
```

###### Secrets

Let a list of secrets required by the request file.

```shell
reqlang parse ./examples/valid/post.reqlang | jq '.secrets'
```

```json
["super_secret_value"]
```

###### Config Location In Request File

Get the span of the config, if defined, in the request file. Otherwise it's `null`.

```shell
reqlang parse ./examples/valid/post.reqlang | jq '.full.config[1]'
```

```json
{
  "start": 0,
  "end": 204
}
```

###### Request Location In Request File

Get the span of the request in the request file.

```shell
reqlang parse ./examples/valid/post.reqlang | jq '.full.request[1]'
```

```json
{
  "start": 208,
  "end": 388
}
```

###### Response Location In Request File

Get the span of the response, if defined, in the request file. Otherwise it's `null`.

```shell
reqlang parse ./examples/valid/post.reqlang | jq '.full.response[1]'
```

```json
null
```

###### Ref Locations In Request File

Get the span of all the template references (variables, prompts, secrets, providers), if defined, in the request file.

```shell
reqlang parse ./examples/valid/post.reqlang | jq '.full.refs'
```

```json
[
  [
    {
      "Provider": "env"
    },
    {
      "start": 208,
      "end": 388
    }
  ],
  [
    {
      "Variable": "test_value"
    },
    {
      "start": 208,
      "end": 388
    }
  ],
  [
    {
      "Prompt": "prompt_value"
    },
    {
      "start": 208,
      "end": 388
    }
  ],
  [
    {
      "Secret": "super_secret_value"
    },
    {
      "start": 208,
      "end": 388
    }
  ]
]
```

##### Validation Errors

If the request file is invalid, a list of errors will be returned instead.

```shell
reqlang parse examples/invalid/empty.reqlang
```

```json
[
  {
    "range": {
      "start": {
        "line": 0,
        "character": 0
      },
      "end": {
        "line": 0,
        "character": 0
      }
    },
    "severity": 1,
    "message": "ParseError: Request file is an empty file"
  }
]
```

#### AST

Produce an AST for a request file.

```
Usage: reqlang ast <path>

Arguments:
  <path>  Path to request file

Options:
  -h, --help  Print help
```

##### Examples

```shell
reqlang ast examples/valid/as_markdown.reqlang
```

```json
[
  [
    {
      "Comment": "# Request Files Are Markdown Files\n\nAnything outside of the config, request, or response code blocks is treated as markdown. This lets you document your request files in a way that is easy to read and understand.\n\n## Config\n\nPrompt the user for the `status_code` to return.\n\n"
    },
    {
      "start": 0,
      "end": 275
    }
  ],
  [
    {
      "ConfigBlock": [
        "[prompts]\n# Status code the response will return\nstatus_code = \"\"",
        {
          "start": 286,
          "end": 352
        }
      ]
    },
    {
      "start": 275,
      "end": 355
    }
  ],
  [
    {
      "Comment": "\n\n## Request\n\nThis will respond with the prompted `status_code`.\n\n"
    },
    {
      "start": 355,
      "end": 421
    }
  ],
  [
    {
      "RequestBlock": [
        "GET https://httpbin.org/status/{{?status_code}} HTTP/1.1",
        {
          "start": 433,
          "end": 490
        }
      ]
    },
    {
      "start": 421,
      "end": 493
    }
  ]
]
```

##### Filtering

###### Comments

```shell
reqlang ast examples/valid/as_markdown.reqlang | jq 'map(select(.[0] | has("Comment")))'
```

```json
[
  [
    {
      "Comment": "# Request Files Are Markdown Files\n\nAnything outside of the config, request, or response code blocks is treated as markdown. This lets you document your request files in a way that is easy to read and understand.\n\n## Config\n\nPrompt the user for the `status_code` to return.\n\n"
    },
    {
      "start": 0,
      "end": 275
    }
  ],
  [
    {
      "Comment": "\n\n## Request\n\nThis will respond with the prompted `status_code`.\n\n"
    },
    {
      "start": 355,
      "end": 421
    }
  ]
]
```

#### Export

Parse and template the request file then export it in different formats.

```
Usage: reqlang export [OPTIONS] <path>

Arguments:
  <path>  Path to request file

Options:
  -e, --env <env>         Resolve with an environment
  -P, --prompt <prompts>  Pass prompt values to resolve with
  -S, --secret <secrets>  Pass secret values to resolve with
  -f, --format <format>   Format to export [default: json] [possible values: http, curl, json, body]
  -h, --help              Print help
```

##### Examples

###### JSON

```shell
reqlang export examples/valid/status_code.reqlang --prompt status_code=200 --format json

# This is the same thing
reqlang export examples/valid/status_code.reqlang --prompt status_code=200
```

```json
{
  "verb": "GET",
  "target": "https://httpbin.org/status/200",
  "http_version": "1.1",
  "headers": [],
  "body": ""
}
```

###### HTTP Request Message

```shell
reqlang export examples/valid/status_code.reqlang --prompt status_code=201 --format http
```

```
GET https://httpbin.org/status/201 HTTP/1.1
```

##### Curl command

```shell
reqlang export examples/valid/status_code.reqlang --prompt status_code=400 --format curl
```

```shell
curl https://httpbin.org/status/400 --http1.1 -v
```

##### Body Text

```shell
reqlang export examples/valid/base64decode.reqlang --format body
```

```
HTTPBIN is awesome

```

##### Validation Errors

If the request file is invalid or there were errors templating, a list of errors will be returned instead.

```shell
reqlang export examples/invalid/empty.reqlang
```

```json
[
  {
    "range": {
      "start": {
        "line": 0,
        "character": 0
      },
      "end": {
        "line": 0,
        "character": 0
      }
    },
    "severity": 1,
    "message": "ParseError: Request file is an empty file"
  }
]
```

### CLI in Docker

The `reqlang` CLI can be run from a docker image.

#### Building

```shell
docker build -t reqlang:0.1.0 .
```

#### Running

A directory of request files can be mounted inside the container's `/usr/local/src` directory to make them accessible.

```shell
docker run --rm --read-only \
    -v "/$PWD/examples":/usr/local/src/examples:ro \
    reqlang:0.1.0 \
    export \
    ./examples/valid/delay.reqlang \
    -f curl \
    -P seconds=5 | bash
```

```
# HTTP/1.1 201 CREATED
# Date: Sat, 14 Dec 2024 19:20:26 GMT
# Content-Type: text/html; charset=utf-8
# Content-Length: 0
# Connection: keep-alive
# Server: gunicorn/19.9.0
# Access-Control-Allow-Origin: *
# Access-Control-Allow-Credentials: true
```

### VS Code

The [VS Code extension](./vsc/#readme) acts as an in-editor REST client.

![VS Code Extension Screenshot](./vsc/screenshot.png)

### Desktop Client

The [desktop client](./reqlang-client) is a very simple GUI written in [egui](https://github.com/emilk/egui). It's mostly used for testing.

![GUI Client Screenshot](./reqlang-client/screenshot.png)

### Web Client

The [web client](./reqlang-web-client/) is a React app powered by a Rust API.

```shell
reqlang-web-client

# Server is running! http://localhost:3000
```

#### Port

The port defaults to a random open port but it can be set using the `REQLANG_WEB_CLIENT_PORT` environment variable.

## Contributing

Please see [CONTRIBUTING.md](./CONTRIBUTING.md) for details on how to contribute.

## Development Log

You can follow the development in this [Bluesky thread](https://bsky.app/profile/testingrequired.com/post/3lcftvxbp622b).

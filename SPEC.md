# Request Language Specification

## Objectives

Request Language (Reqlang/reqlang) aims to be an easy to read and write document format for encoding an HTTP request, its associated variables, prompts & secrets as well as an optional expected HTTP response.

HTTP requests and responses are written as [HTTP Messages](https://developer.mozilla.org/en-US/docs/Web/HTTP/Messages) outside of templated values.

Secret values are declared let individual implementations handle how secret values are obtained.

## Example

```reqlang

[envs.default]

---
GET https://example.com HTTP/1.1

```

## Structure

Reqlang is a multi-document file format. A config document where variables, prompts, and secrets are declared. Then a request document containing a templated HTTP Request message. Then an optional response document containing an assertion HTTP Response message.

Documents are delimited using the regex pattern: `^---\n$`.

## Configuration Document

The configuration document is where environment names, variables, prompts, and secrets are declared. The configuration itself is written in [TOML](https://toml.io/en/).

### Environments

Environments are names used to define enviroment specific variable values.

```reqlang

[envs.dev]
base_url = "https://dev.example.com"

[envs.prod]
base_url = "https://dev.example.com"

---
GET {{:base_url}}/api/path/to/endpoint HTTP/1.1

```

#### Requirement

A reqlang file must have at least one environment name declared regardless if any variables are defined. This `default` environment will be provided automatically in the future.

```reqlang

[envs.default]

---
GET https://example.com HTTP/1.1

```

### Variables

Environment based values that are declared in the `vars` array and defined in the `[envs.ENV]` tables. They are referenced using the `{{:variable_name}}` format.

```reqlang

vars = ["base_url"]

[envs.dev]
base_url = "https://dev.example.com"

[envs.prod]
base_url = "https://dev.example.com"

---
GET {{:base_url}}/api/path/to/endpoint HTTP/1.1

```

#### Specs

- Declared variables must have a value defined in all environments
- Declared variables must used/referenced in either the config, request, or response documents.
- Referenced variables must be declared.

### Prompts

User provided values at time of request execution with an optional default value. They are defined in the `[prompts]` table and referenced using the `{{?prompt_name}}` format.

```reqlang

[envs.default]

[prompts]
prompt_value = ""

---
GET https://example.com/?value={{?prompt_value}} HTTP/1.1

```

#### Specs

- Declared prompts must used/referenced in either the config, request, or response documents.
- Referenced prompts must be defined.

### Secrets

Secret values that are declared in the `secrets` array. How their values are pulled are outside of reqlang file's scope. They are referenced using the `{{!secret_name}}` format.

```reqlang

[envs.default]

secrets = ["secret_value"]

---
GET https://example.com/?value={{!secret_value}} HTTP/1.1

```

#### Specs

- Declared secrets must used/referenced in either the config, request, or response documents.
- Referenced secrets must be declared.

## Request Document

The request document contains an [HTTP Request Message](https://developer.mozilla.org/en-US/docs/Web/HTTP/Messages#http_requests) with possible references to variables, prompts, secrets, or provided values.

```reqlang

vars = ["test_value"]
secrets = ["super_secret_value"]

[prompts]
prompt_value = ""

[envs.test]
test_value = "test_value"

[envs.prod]
test_value = "prod_value"

[envs.local]
test_value = "local_value"

---
POST https://httpbin.org/post HTTP/1.1

{
  "env": "{{@env}}",
  "value": "{{:test_value}}",
  "prompted_value": "{{?prompt_value}}",
  "secret_value": "{{!super_secret_value}}"
}
```

### Specs

- An untemplated request document must still be a valid HTTP Request Message

## Provided Values

In addition to variables, prompts, and secrets there are also provided values that are "provided" by the implementation/client.

One example is `{{@env}}` provider value. It templates in the environment name selected at the time of request execution.

## Response Document

The response document contains an [HTTP Response Message](https://developer.mozilla.org/en-US/docs/Web/HTTP/Messages#http_responses) with possible references to variables, prompts, secrets, or provided values. This response message defined the expected response from the request.

This document is optional.

```reqlang

vars = ["test_value"]
secrets = ["super_secret_value"]

[prompts]
prompt_value = ""

[envs.test]
test_value = "test_value"

[envs.prod]
test_value = "prod_value"

[envs.local]
test_value = "local_value"

---
POST https://httpbin.org/post HTTP/1.1
content-type: application/json
accept: application/json

{
  "env": "{{@env}}",
  "value": "{{:test_value}}",
  "prompted_value": "{{?prompt_value}}",
  "secret_value": "{{!super_secret_value}}"
}
---
HTTP/1.1 200 OK
access-control-allow-credentials: true
access-control-allow-origin: https://httpbin.org
content-length: {{@wildcard}}
content-type: application/json
date: {{@wildcard}}
server: gunicorn/19.9.0

{
  "args": {},
  "data": "",
  "files": {},
  "form": {},
  "headers": {
    "content-type": "application/json",
    "accept": "application/json",
  },
  "json": null,
  "origin": {{@wildcard}},
  "url": "https://httpbin.org/post"
}
```

### Specs

- An untemplated response document must still be a valid HTTP Request Message

## Matching

### What You See Is What You Match (WYSIWYM)

Only the headers present in the response document are matched against. The response body can also be partially matched.

### Wildcard

The `{{@wildcard}}` provider reference will match anything from it's start to end.

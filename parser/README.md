# Steps

## Example Request File

```
#!/usr/bin/env reqlang

Example request file
---
GET / HTTP/1.1
host: {{:base_url}}
x-test: {{?test_value}}
x-api-key: {{!api_key}}

---
HTTP/1.1 200 OK

---
vars = ["base_url"]

[envs.dev]
base_url = "https://dev.example.com"

[envs.prod]
base_url = "https://example.com"

prompts = ["test_value"]

secrets = ["api_key"]
---
```

## Splitter

The request file is split using the `---` delimiter in to the following documents:

- Shebang and request description
- Request message
- Response message (optional)
- Configuration (optional)
- Empty string

## Parser

The parser runs each document through a parsing process. It returns an `UnresolvedRequestFile`.

## Resolver

The resolver applies an environment name (`env`) to the `envs` values to map `vars` in to `HashMap<String, String>`. It also accepts both `prompts` and `secrets` values as `Hash<String, String>`. The `Request` and optional `Response` are also parsed. It returns a `ResolvedRequestFile`.

Template placeholders e.g. `{{:var_name}}`, `{{?prompt_name}}`, `{{$secret_name}}` are still present.

## Templater

The templater replaces template placeholders with their resolved values from `vars`, `prompts`, or `secrets`. It returns a `TemplatedRequestFile`.

# (Req)uest (Lang)uage

A format for defining http/s requests.

## Examples

### Request

Requests are written as an HTTP request messages.

```reqlang
#!/usr/bin/env reqlang
---
GET https://example.com HTTP/1.1
```

### Response Assertions

Responses are treated as an assertion and are written as an HTTP response message.

```reqlang
#!/usr/bin/env reqlang
---
GET https://example.com HTTP/1.1
---
HTTP/1.1 200 OK
```

### Variables, Evironmental Values, & Template References

Requests and responses support templating be declaring variables and defining environment specific values.

```reqlang
#!/usr/bin/env reqlang

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
#!/usr/bin/env reqlang

vars = ["base_url"]

[envs.dev]
base_url = "https://dev.example.com"

[envs.prod]
base_url = "https://example.com"

[prompts]
example_id = ""
---
GET {{:base_url}}/?id={{?example_id}} HTTP/1.1
```

### Secrets

Secrets are declared but their values are supplied at template time.

```reqlang
#!/usr/bin/env reqlang

vars = ["base_url"]
secrets = ["api_key"]

[envs.dev]
base_url = "https://dev.example.com"

[envs.prod]
base_url = "https://example.com"

[prompts]
example_id = ""
---
GET {{:base_url}}/?id={{?example_id}} HTTP/1.1
x-api-key: {{!api_key}}
```

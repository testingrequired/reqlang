# Request Language Specification

## Requests Collection

A collection or project is a directory with a `reqlang.json` and a collection of `*.reqlang` request files in the same directory.

### reqlang.json

```json
{
  "name": "blogapi"
  "description": "An example collection for a fake blog api",
  "version": "0.1.0",
  "envs": ["local", "dev", "qa", "prod"],
  "vars": [
    "base_url"
  ],
  "env": {
    "local": {
      "base_url": ""
    },
    "dev": {
      "base_url": ""
    },
    "qa": {
      "base_url": ""
    },
    "prod": {
      "base_url": ""
    },
  }
}
```

### reqlang.toml

```toml
name = "blogapi"
description = "An example collection for a fake blog api"
version = "0.1.0"
envs = ["local", "dev", "qa", "prod"]
vars = ["base_url"]

[env]

[env.local]
base_url = ""

[env.dev]
base_url = ""

[env.qa]
base_url = ""

[env.prod]
base_url = ""
```

#### name

The name of the request collection. Must conform the this pattern: `[a-z]+[_a-zA-Z0-9]*`.

## Request File

Request files define several things about a request:

- The HTTP request message itself with templated values
- An optional HTTP response message assertions with templated values & wildcards
- Declarations and definitions of the variables, prompts, and secrets used in the request file
- Environment names and environmental values for variables
- Output values extracted from the response

### Request Id/File Name

Each request in reqlang is defined in a file: `:id.reqlang` where `:id` becomes the `id` of the request. The filename/`id` must conform the this pattern: `[a-z]+[_a-zA-Z0-9]*`.

#### Fully Qualified Id

Within a collection a request's fully qualified name is a combination of the collection's `name` and the request file's file name. e.g. `blogapi::get_user_posts_by_tag`

### Format

The request file is split in to several documents using `---`:

- Head (shebang, description)
- Config (variables, envs, prompts, secrets, auth, output)
- Request Message
- Response Message Assertion
- Empty

```
#!/usr/bin/env reqlang
/// get_user_posts_by_tag.reqlang

Get a users posts by tag/s
---
/// Variables store default and environment based values
/// Template syntax: `{{var:var_name}}`
/// Shortcut syntax: `{{:var_name}}`
vars {
  client_id,
  access_token_uri
}

/// Environment names are declared here.
/// Environmental data is also defined here.
envs {
  local {
    client_id = "ba44...",
    access_token_uri = "http://dev.example.com/token"
  },
  dev {
    client_id = "ba44...",
    access_token_uri = "http://dev.example.com/token"
  },
  qa {
    client_id = "ff12...",
    access_token_uri = "https://qa.example.com/token"
  },
  prod {
    client_id = "012b...",
    access_token_uri = "https://example.com/token"
  }
}

/// Prompts are values provided by the user at request time
/// Template syntax: `{{prompt:prompt_name}}`
/// Shortcut syntax: `{{!prompt_name}}`
prompts {
  user_id,
  tagged_with
}

/// Secrets are declared in the request
/// The values are provided at request time by the runtime
/// Template syntax: `{{secret:secret_name}}`
/// Shortcut syntax: `{{$secret_name}}`
secrets {
  client_secret
}

auth {
  oauth2 {
    grant = "client",
    access_token_uri = {{:access_token_uri}},
    client_id = {{:client_id}}
    client_secret = {{$client_secret}}
    scopes = "profile"
  }
}

/// Extract values from the response
outputs {
  ids: body {
    json_path($[*].id)
  }
}
---
/// Define requests using http request messages and templating
GET {{:base_url}}/users/{{!user_id}}/posts?tags={{!tagged_with}} HTTP/1.1
{{@auth.header}}
content-type: application/json

---
/// Assert against the response
200 OK
content-type: application/json
{{*}} // Exclude the rest of the headers

[
  {
    "id": {{*}}
    "title": {{*}}
  },
  {
    "id": {{*}}
    "title": {{*}}
  }
]
---
```

### Shebang

The (optional) shebang that should be used in the Head document is `#!/usr/bin/env reqlang`.

### Validation

#### Minimum Documents

- Head
- Config
- Request Message
- Empty

#### All referenced variables are declared and defined.

All `{{:var_name}}` references must be declared in `Config.vars` and be defined in `Config.vars` or `Config.envs[env]`.

#### Warning: Variables defined in one env should be defined in all envs unless variable has a default value in it's declaration

Defining an enviromental value for a variable in one env should be defined in all envs unless the variable was declared with a default value. Will produce an warning if not.

#### Environment names must match template and collection

The env names declared in `Config.envs` must match the envs in `Config.template` (if defined) and the collection's env names in `reqlang.json`.

#### Defined environmental values must have a variable declaration

All `Config.envs[env][var_name]` must be declared in `Config.vars[var_name]`.

#### All referenced prompts are declared.

All `{{!prompt_name}}` references must be declared in `Config.prompts`.

#### All referenced secrets are declared.

All `{{$secret_name}}` references must be declared in `Config.secrets`.

#### Warning: All declared variables, prompts, and secrets should be referenced

All `{{:var_name}}`, `{{!prompt_name}}`, `{{$secret_name}}` should be referenced/used. Will produce a warning if not.

#### Must end with `---\n`

The request file must end with `---\n` leaving the Empty document at the end of the file.

#### Warning: Non standard shebang

If a shebang is present in the Head document is should be `#!/usr/bin/env reqlang`. Will produce a warning if not.

## Template Request Files

Template request files are very similar to request files with a few key differences:

- No shebang
- No request message
- No response message assertion

### Request Id/File Name

Each request template in reqlang is defined in a file: `:id.template.reqlang` where `:id` becomes the `id` of the request template. The filename/`id` must conform the this pattern: `[a-z]+[_a-zA-Z0-9]*`

### Format

The request template file is split in to several documents using `---`:

- Empty
- Config (variables, envs, prompts, secrets, auth)
- Empty

```
/// ./base.template.reqlang
---
vars {
  client_id,
  access_token_uri
}

envs {
  local {
    client_id = "ba44...",
    access_token_uri = "http://dev.example.com/token"
  },
  dev {
    client_id = "ff33...",
    access_token_uri = "http://dev.example.com/token"
  },
  qa {
    client_id = "12da...",
    access_token_uri = "https://qa.example.com/token"
  },
  prod {
    client_id = "d32f...",
    access_token_uri = "https://example.com/token"
  }
}

secrets {
  client_secret
}

auth {
  oauth2 {
    grant = "client",
    access_token_uri = {{:access_token_uri}},
    client_id = {{:client_id}}
    client_secret = {{$client_secret}}
    scopes = "profile"
  }
}

headers {
  authentication = "Bearer {{@auth.oauth2.access_token}}"
}
---
```

### Shebang

No shebang should be included in template request files since they don't include a request message.

### Validation

#### Minimum Documents

- Empty
- Config
- Empty

#### All referenced variables are declared and defined.

All `{{:var_name}}` references must be declared in `Config.vars` and be defined in `Config.vars` or `Config.envs[env]`.

#### Warning: Variables defined in one env should be defined in all envs unless variable has a default value in it's declaration

Defining an enviromental value for a variable in one env should be defined in all envs unless the variable was declared with a default value. Will produce an warning if not.

#### Defined environmental values must have a variable declaration

All `Config.envs[env][var_name]` must be declared in `Config.vars[var_name]`.

#### All referenced prompts are declared.

All `{{!prompt_name}}` references must be declared in `Config.prompts`.

#### All referenced secrets are declared.

All `{{$secret_name}}` references must be declared in `Config.secrets`.

#### Warning: All declared variables, prompts, and secrets should be referenced

All `{{:var_name}}`, `{{!prompt_name}}`, `{{$secret_name}}` should be referenced/used. Will produce a warning if not.

#### Must end with `---\n`

The template request file must end with `---\n` leaving the Empty document at the end of the file.

#### No shebang

Template files don't include a request message or aren't actionable. They should not include a shebang to reflect this.

#### Multiple `Config.auth` entries

Only one `Config.auth` entry can be present in a request at a time.

#### Tempate must be a template

`Config.template/s` must be a template file. Will produce an error if it's a request or invalid file.

### Merging

Request files should declare their (optional) template using `Config.template/s`. When `Config.template/s` is defined the `Config` document of both requests are merged.

#### Config.templates/Config.template

Template files can extend other template files. The extendee's `Config.template/s` is not overridden when merging.

#### Config.vars

`Config.vars` are merged by applying the request file's `Config.vars` on top of the template's `Config.vars`. An error is produced if the keys collide.

#### Config.envs

`Config.envs` env name keys should match each other and the var names should collide already.

#### Config.secrets

`Config.secrets` are merged by applying the request file's `Config.secrets` on top of the template's `Config.secrets`.

#### Config.prompts

`Config.prompts` are merged by applying the request file's `Config.prompts` on top of the template's `Config.prompts`. An error is produced if the keys collide.

#### Config.auth

???

#### Config.outputs

???

#### Config.headers

Request templates can define headers that will be appended to the extendee's request message. Extendee's that override template header's will generate a warning for visibility.

### Extendee Request

`template "blogapi::base.template"` is sugar for `templates ["blogapi::base.template"]`

```
#!/usr/bin/env reqlang
/// get_user_posts_by_tag.reqlang

Get a users posts by tag/s
---
template "blogapi::base.template"

prompts {
  user_id,
  tagged_with
}

outputs {
  ids = body {
    json_path($[*].id)
  }
}
---
GET {{:base_url}}/users/{{!user_id}}/posts?tags={{!tagged_with}} HTTP/1.1
content-type: application/json

---
200 OK
content-type: application/json

[
  {
    "id": {{*}}
    "title": {{*}}
  },
  {
    "id": {{*}}
    "title": {{*}}
  }
]
---
```

# opi

OpenAPI 3.0 / 3.1 / 3.2 → TypeScript types, plus a typed `fetch` / XHR client for them.

```bash
pnpm add -D github:Pingid/opi#pkg
```

```bash
opi pets.yaml > pets.ts      # default rules, to stdout
opi pets.yaml pets.ts        # or to a file
opi https://petstore3.swagger.io/api/v3/openapi.json   # or a URL
opi --init > opi.yaml        # write the default config to edit
opi -c opi.yaml              # spec from `source`, one file per key
opi pets.yaml -c opi.yaml    # spec from the command line instead
opi --completions zsh
```

## Example

<summary><code>pets.yaml</code></summary>

```yaml
openapi: 3.1.0
info: { title: Pets, version: '1' }
paths:
  /pets/{id}:
    get:
      summary: Get a pet
      parameters:
        - { name: id, in: path, schema: { type: integer } }
      responses:
        '200':
          description: The pet
          content:
            application/json:
              schema: { $ref: '#/components/schemas/Pet' }
        '404': { description: Not found }
  /pets:
    post:
      summary: Add a pet
      requestBody:
        required: true
        content:
          application/json:
            schema: { $ref: '#/components/schemas/Pet' }
      responses:
        '201': { description: Created }
components:
  schemas:
    Pet:
      type: object
      required: [id, name]
      properties:
        id: { type: integer }
        name: { type: string }
        photo: { type: string, format: binary }
```

</details>

`opi.yaml`

```yaml
source: ./pets.yaml
formats: { binary: Blob }

./pets.ts:
  - each: schema
    emit: { type: '{name}' }

  - each: route
    emit:
      type: '{method}{path}'
      fields: { method: '{method}', request: '{request}', response: '{response}' }

  - each: route
    where: { deprecated: false }
    emit:
      type: Routes
      fields:
        '{METHOD} {path}':
          request: "{method}{path}['request']"
          response: "{method}{path}['response']"
```

`opi -c opi.yaml` → `pets.ts`

```ts
export type Pet = {
  id: number
  name: string
  photo?: Blob
}
/** @summary Get a pet */
export type GetPetsById = {
  method: 'get'
  request: {
    body?: never
    contentType?: never
    params: {
      id: number
    }
    query?: never
    headers?: never
    cookies?: never
  }
  response:
    | {
        /** @description The pet */
        status: 200
        contentType: 'application/json'
        body: Pet
      }
    | {
        /** @description Not found */
        status: 404
        contentType: null
        body?: never
      }
}
/** @summary Add a pet */
export type PostPets = {
  method: 'post'
  request: {
    body: Pet
    contentType: 'application/json'
    params?: never
    query?: never
    headers?: never
    cookies?: never
  }
  response: {
    /** @description Created */
    status: 201
    contentType: null
    body?: never
  }
}
export type Routes = {
  /** @summary Get a pet */
  'GET /pets/{id}': {
    request: GetPetsById['request']
    response: GetPetsById['response']
  }
  /** @summary Add a pet */
  'POST /pets': {
    request: PostPets['request']
    response: PostPets['response']
  }
}
```

## Client

```ts
import { Fetch, Xhr } from '@pingid/opi'
import type { Routes } from './pets.ts'

const api = Fetch.client<Routes>({ baseUrl: 'https://pets.example' })

const res = await api('GET /pets/{id}', { params: { id: 1 } })
if (res.status === 200) {
  const pet = await res.json() // Pet
}

await api('POST /pets', { contentType: 'application/json', body: { id: 2, name: 'Rex' } })

const xhr = Xhr.client<Routes>({ baseUrl: 'https://pets.example' })

const req = xhr('POST /pets', { contentType: 'application/json' })
req.upload.onprogress = (e) => console.log(e.loaded / e.total)
req.send({ id: 2, name: 'Rex' })
const { status } = await req.result() // 201
```

## Rules

`each` picks items, `where` filters them, `emit` writes a declaration per `type` name. Items with the same `type` merge into one declaration. The examples below also have `- each: schema` so `Pet` is named rather than inlined.

| `each`                | variables                                                   |
| --------------------- | ----------------------------------------------------------- |
| `schema`              | `{name}` `{schema}` `{deprecated}`                          |
| `route`               | `{method}` `{path}` `{request}` `{response}` `{deprecated}` |
| `{ route: request }`  | `{content_type}` `{body}` + the route's                     |
| `{ route: response }` | `{status}` `{content_type}` `{schema}` + the route's        |

`{method}` get · `{METHOD}` GET · `{Method}` Get · `{path}` /pets/{id} · `{Path}` PetsById

```yaml
- each: route
  emit: { type: Paths, fields: '{path}' }
```

```ts
export type Paths = '/pets/{id}' | '/pets'
```

```yaml
- each: route
  emit: { type: Methods, fields: { '{path}': '{METHOD}' } }
```

```ts
export type Methods = {
  /** @summary Get a pet */
  '/pets/{id}': 'GET'
  /** @summary Add a pet */
  '/pets': 'POST'
}
```

```yaml
- each: route
  emit: { type: Names, fields: { '{path}': "'{Method}{Path}'" } } # quoted: a string literal
```

```ts
export type Names = {
  /** @summary Get a pet */
  '/pets/{id}': 'GetPetsById'
  /** @summary Add a pet */
  '/pets': 'PostPets'
}
```

```yaml
- each: route
  where: { method: get }
  emit: { type: '{Method}{Path}Response', fields: '{response}' }
```

```ts
/** @summary Get a pet */
export type GetPetsByIdResponse =
  | {
      /** @description The pet */
      status: 200
      contentType: 'application/json'
      body: Pet
    }
  | {
      /** @description Not found */
      status: 404
      contentType: null
      body?: never
    }
```

```yaml
- each: { route: response }
  where: { response: { status: 200 } }
  emit: { type: Ok, fields: { '{METHOD} {path}': '{schema}' } }
```

```ts
export type Ok = {
  /** @description The pet */
  'GET /pets/{id}': Pet
}
```

```yaml
- each: { route: request }
  emit: { type: Bodies, fields: { '{path}': { '{content_type}': '{body}' } } }
```

```ts
export type Bodies = {
  '/pets': {
    'application/json': Pet
  }
}
```

```yaml
where:
  deprecated: false
  method: get
  path: /pets/{id}
  request: { content_type: application/json }
  response: { status: 200, content_type: application/json }
```

## Config

```yaml
source: ./pets.yaml # relative to this file, or a URL; the CLI argument wins
formats: { binary: Blob } # format → TS type; {} for none
rules: [...] # to the CLI's output argument, or stdout
./pets.ts: [...] # any other key: a file, relative to this file
```

## Development

```bash
cargo test
UPDATE_GOLDEN=1 cargo test   # rewrite opi/tests/fixtures/*.ts and config.schema.json
```

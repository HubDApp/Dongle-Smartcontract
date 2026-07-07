# Language Metadata

Dongle metadata documents can expose language information off-chain so
frontends and indexers can filter projects and reviews by locale without adding
new on-chain storage.

## Project Metadata

Use `language` for the primary language of the project metadata document. Use
`supportedLanguages` for the languages a project supports in its UI,
documentation, or community materials.

```json
{
  "language": "en",
  "supportedLanguages": ["en", "es", "fr", "pt-BR"]
}
```

## Review Metadata

Use `language` on review CID documents for the language of the review text.

```json
{
  "version": "1.0.0",
  "text": "Tres bon projet.",
  "language": "fr"
}
```

## Accepted Format

Language values use a conservative BCP 47-compatible pattern:

```text
^[a-z]{2,3}(-[A-Za-z0-9]{2,8})*$
```

Accepted examples:

- `en`
- `fr`
- `es`
- `pt-BR`
- `zh-Hant`
- `sr-Latn-RS`

Rejected examples:

- `EN`
- `english`
- `pt_BR`
- `fr-`
- `x`

## Privacy And Indexing Notes

Language metadata should describe the content language only. Do not infer or
publish a reviewer's nationality, location, or identity from language choice.
Indexers should treat missing language values as unknown and should not reject
legacy metadata documents that predate these fields.

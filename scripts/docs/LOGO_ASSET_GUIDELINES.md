# Logo Asset Guidelines

This document defines the recommended standards for project logo assets stored
via IPFS CID references in the Dongle smart contract.

## Supported Formats

| Format | MIME Type         | Recommended |
|--------|-------------------|-------------|
| PNG    | `image/png`       | Yes         |
| SVG    | `image/svg+xml`   | Yes         |
| WebP   | `image/webp`      | Yes         |
| JPEG   | `image/jpeg`      | Acceptable  |
| GIF    | `image/gif`       | No (static only) |

**PNG** and **SVG** are strongly recommended. SVG scales to any resolution
without quality loss. PNG provides lossless raster output with transparency.

## Recommended Dimensions

| Use Case         | Minimum    | Recommended | Maximum    |
|------------------|------------|-------------|------------|
| Square logo      | 128 x 128  | 512 x 512   | 1024 x 1024 |
| Aspect ratio     | 1:1        | 1:1          | 1:1        |

- Logos **must** be square (1:1 aspect ratio).
- Provide at least 512 x 512 pixels for crisp rendering across devices.
- Avoid upscaled raster images that appear blurry at target sizes.

## Maximum File Size

| Format | Max Size |
|--------|----------|
| SVG    | 100 KB   |
| PNG    | 500 KB   |
| WebP   | 500 KB   |
| JPEG   | 500 KB   |

These are client-side recommendations. The contract stores only the IPFS CID
(up to 128 characters) and does **not** enforce image dimensions or file size
on-chain.

## On-chain vs Off-chain Responsibility

The smart contract validates only:
- CID format (valid IPFS CIDv0 or CIDv1, max 128 characters)
- CID is non-empty when provided

**All other validation is the responsibility of client applications:**
- Image format and MIME type
- Dimensions and aspect ratio
- File size limits
- Content safety and moderation
- Accessibility (alt text, contrast)

Clients **should** validate logo assets before displaying them to users and
reject assets that do not conform to these guidelines.

## Content Safety

Since logo content is stored off-chain on IPFS, it cannot be moderated by the
contract. Client implementations should:

1. **Validate MIME type** before rendering (do not trust file extensions).
2. **Apply Content Security Policy (CSP)** headers when serving images.
3. **Sanitize SVGs** — SVG files can contain embedded scripts. Use a sanitizer
   like DOMPurify or render SVGs as `<img>` tags (not inline `<svg>`).
4. **Implement reporting** — allow users to flag inappropriate logos using the
   contract's `report_project` functionality.
5. **Cache defensively** — IPFS content is immutable by CID, but gateways may
   serve different content for the same CID under adversarial conditions. Pin
   and verify content on your own IPFS node when possible.

## Metadata CID Structure

When using `metadata_cid`, the referenced JSON document should include a `logo`
field alongside other project metadata:

```json
{
  "logo": {
    "cid": "QmExample...",
    "format": "image/png",
    "width": 512,
    "height": 512,
    "size_bytes": 45230
  },
  "banner": {
    "cid": "QmBanner...",
    "format": "image/png",
    "width": 1200,
    "height": 630
  }
}
```

## Examples

### Setting a logo CID during registration

```rust
let params = ProjectRegistrationParams {
    owner: owner_address,
    name: String::from_str(&env, "My Project"),
    slug: String::from_str(&env, "my-project"),
    description: String::from_str(&env, "A cool project"),
    category: String::from_str(&env, "DeFi"),
    website: None,
    logo_cid: Some(String::from_str(&env, "QmYourLogoCidHere...")),
    metadata_cid: None,
    tags: None,
    social_links: None,
    launch_timestamp: None,
    bounty_url: None,
};

client.register_project(&params);
```

### Updating a logo CID

```rust
let params = ProjectUpdateParams {
    project_id: 1,
    caller: owner_address,
    name: None,
    slug: None,
    description: None,
    category: None,
    website: None,
    logo_cid: Some(String::from_str(&env, "QmNewLogoCidHere...")),
    metadata_cid: None,
    tags: None,
    social_links: None,
    launch_timestamp: None,
    bounty_url: None,
};

client.update_project(&params);
```

> **Note:** For verified projects, logo CID updates are frozen to prevent
> impersonation. The project must be unverified before the logo can be changed.

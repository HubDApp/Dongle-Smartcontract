# Optional Fields Validation - Quick Reference Guide

## Overview
This guide provides quick reference for using the new optional field validation functions in the Dongle smart contract.

## Validation Functions

### 1. URL Validation

```rust
// Function signature
pub fn is_valid_url(url: &String) -> bool

// Usage example
use crate::utils::Utils;

let url = String::from_slice(&env, b"https://example.com");
if Utils::is_valid_url(&url) {
    // URL is valid
} else {
    // URL is invalid
}
```

**Valid Patterns:**
- `https://example.com` ✅
- `http://www.example.com` ✅
- `https://example.com/path` ✅

**Invalid Patterns:**
- `example.com` ❌ (no protocol)
- `ftp://example.com` ❌ (unsupported protocol)
- `https://` + 300 chars ❌ (too long)

---

### 2. IPFS CID Validation

```rust
// Function signature
pub fn is_valid_ipfs_cid(cid: &String) -> bool

// Usage example
use crate::utils::Utils;

let cid = String::from_slice(&env, b"QmXwGvnZ9zLo3vwU5r8G8Q8QvH1qvKrW9d8X8X8X8X8X8X8X8");
if Utils::is_valid_ipfs_cid(&cid) {
    // CID is valid
} else {
    // CID is invalid
}
```

**Valid Patterns:**
- `QmXwGvnZ9zLo3vwU5r8G8Q8QvH1qvKrW9d8X8X8X8X8X8X8X8` ✅ (CIDv0, Qm)
- `bagaaierahlcq5yduxj6ahjw5qhuhm47aaelzb3dqw67pyzwsaaaa` ✅ (CIDv1)

**Invalid Patterns:**
- `QmShort` ❌ (too short)
- `QnXwGvnZ9zLo3vwU5r8G8Q8QvH1qvKrW9d8X8X8X8X8X8X8X8` ❌ (wrong pattern)
- `Qm...` + 300 chars ❌ (too long)

---

### 3. Optional Website Validation

```rust
// Function signature
pub fn validate_optional_website(website: &Option<String>) -> Result<(), ContractError>

// Usage examples
use crate::utils::Utils;

// Case 1: None - always valid
let result = Utils::validate_optional_website(&None);
assert!(result.is_ok()); // ✅

// Case 2: Valid URL - passes validation
let url = String::from_slice(&env, b"https://example.com");
let result = Utils::validate_optional_website(&Some(url));
assert!(result.is_ok()); // ✅

// Case 3: Invalid URL - fails validation
let url = String::from_slice(&env, b"invalid");
let result = Utils::validate_optional_website(&Some(url));
assert!(result.is_err()); // ❌ Returns InvalidProjectData
```

---

### 4. Optional Logo CID Validation

```rust
// Function signature
pub fn validate_optional_logo_cid(logo_cid: &Option<String>) -> Result<(), ContractError>

// Usage examples
use crate::utils::Utils;

// Case 1: None - always valid
let result = Utils::validate_optional_logo_cid(&None);
assert!(result.is_ok()); // ✅

// Case 2: Valid CID - passes validation
let cid = String::from_slice(&env, b"QmXwGvnZ9zLo3vwU5r8G8Q8QvH1qvKrW9d8X8X8X8X8X8X8X8");
let result = Utils::validate_optional_logo_cid(&Some(cid));
assert!(result.is_ok()); // ✅

// Case 3: Invalid CID - fails validation
let cid = String::from_slice(&env, b"invalid");
let result = Utils::validate_optional_logo_cid(&Some(cid));
assert!(result.is_err()); // ❌ Returns InvalidProjectData
```

---

### 5. Optional Metadata CID Validation

```rust
// Function signature
pub fn validate_optional_metadata_cid(metadata_cid: &Option<String>) -> Result<(), ContractError>

// Usage pattern identical to validate_optional_logo_cid()
```

---

### 6. Optional Comment CID Validation

```rust
// Function signature
pub fn validate_optional_comment_cid(comment_cid: &Option<String>) -> Result<(), ContractError>

// Usage pattern identical to validate_optional_logo_cid()
```

---

## Integration Examples

### Example 1: Register Project with Optional Fields

```rust
let params = ProjectRegistrationParams {
    owner: owner_address,
    name: project_name,
    description: project_description,
    category: project_category,
    website: Some(String::from_slice(&env, b"https://example.com")),
    logo_cid: Some(String::from_slice(&env, b"QmXwGvnZ9zLo3vwU5r8G8Q8QvH1qvKrW9d8X8X8X8X8X8X8X8")),
    metadata_cid: None, // Optional fields can be None
};

// ProjectRegistry::register_project() automatically validates all optional fields
let project_id = ProjectRegistry::register_project(&env, params)?;
// If validation fails, returns ContractError::InvalidProjectData
```

### Example 2: Update Project with New Optional Fields

```rust
let params = ProjectUpdateParams {
    project_id: 1,
    caller: owner_address,
    name: None, // Not updating
    description: None, // Not updating
    category: None, // Not updating
    website: Some(Some(String::from_slice(&env, b"https://newsite.com"))),
    logo_cid: None, // Not updating
    metadata_cid: None, // Not updating
};

// ProjectRegistry::update_project() automatically validates updated optional fields
let updated_project = ProjectRegistry::update_project(&env, params)?;
// If validation fails, returns ContractError::InvalidProjectData
```

### Example 3: Submit Review with Comment CID

```rust
let comment_cid = String::from_slice(&env, b"QmYwGvnZ9zLo3vwU5r8G8Q8QvH1qvKrW9d8Y8Y8Y8Y8Y8Y8Y8");

// ReviewRegistry::add_review() automatically validates comment CID
ReviewRegistry::add_review(
    &env,
    project_id,
    reviewer_address,
    rating,
    Some(comment_cid),
)?;
// If validation fails, returns ContractError::InvalidProjectData
```

---

## Error Handling

All validation functions that return `Result<(), ContractError>` should be handled with the `?` operator:

```rust
// Using ? operator (recommended)
Utils::validate_optional_website(&website)?; // Propagates error if validation fails

// Manual error handling
match Utils::validate_optional_website(&website) {
    Ok(_) => {
        // Continue processing
    }
    Err(e) => {
        return Err(e); // Return error
    }
}

// Using if let
if let Err(e) = Utils::validate_optional_website(&website) {
    return Err(e);
}
```

---

## Constants Reference

| Constant | Value | Applies To |
|----------|-------|-----------|
| `MAX_WEBSITE_LEN` | 256 | website URLs |
| `MAX_CID_LEN` | 128 | All IPFS CIDs |

Location: `src/constants.rs`

---

## Validation Constraints Summary

### URL Validation
| Constraint | Value |
|-----------|-------|
| Minimum Length | 1 char |
| Maximum Length | 256 chars |
| Required Format | `http://` or `https://` |
| Required Content | At least 1 char after `://` |

### IPFS CID Validation
| Constraint | Value |
|-----------|-------|
| Minimum Length | 46 chars |
| Maximum Length | 128 chars |
| CIDv0 Format | Starts with `Qm` |
| CIDv1 Format | Lowercase alphanumeric |
| Special Chars | Not allowed (except CIDv0 base58) |

---

## Testing Validation Functions

Run the test suite with:

```bash
# All optional field validation tests
cargo test optional_field_validation

# Specific test
cargo test test_valid_https_url

# With output
cargo test optional_field_validation -- --nocapture --test-threads=1
```

---

## Common Pitfalls

❌ **Don't do this:**
```rust
// Forgetting to unwrap Option or handle Result
let is_valid = Utils::is_valid_url(&url); // Returns bool, not Result

// Forgetting the ? operator in Result handling
Utils::validate_optional_website(&website); // Error not propagated
```

✅ **Do this instead:**
```rust
// For bool functions
if Utils::is_valid_url(&url) {
    // Process
}

// For Result functions with ?
Utils::validate_optional_website(&website)?;

// Or with match
match Utils::validate_optional_website(&website) {
    Ok(_) => { /* Process */ },
    Err(e) => { /* Handle error */ }
}
```

---

## FAQ

**Q: What happens if I provide None for an optional field?**  
A: Validation passes. None values are always valid.

**Q: What error is returned on validation failure?**  
A: `ContractError::InvalidProjectData` is returned for all optional field validation failures.

**Q: Can I use these functions outside of registration/update?**  
A: Yes! The validation functions are public and can be called from anywhere in the contract.

**Q: Do I need to call validation functions manually?**  
A: No. If you use `ProjectRegistry::register_project()`, `ProjectRegistry::update_project()`, `ReviewRegistry::add_review()`, or `ReviewRegistry::update_review()`, validation is automatic.

**Q: What's the performance impact?**  
A: Negligible. Validation is O(n) where n is the string length, which is bounded by MAX_WEBSITE_LEN (256) or MAX_CID_LEN (128).

**Q: Are there any breaking changes?**  
A: No. The changes are fully backwards compatible. Existing code continues to work unchanged.

---

## Support

For issues or questions about validation implementation, see:
- [VALIDATION_ENHANCEMENT.md](VALIDATION_ENHANCEMENT.md) - Detailed specifications
- [IMPLEMENTATION_SUMMARY.md](IMPLEMENTATION_SUMMARY.md) - Complete implementation details
- Test file: [src/tests/optional_field_validation.rs](src/tests/optional_field_validation.rs)

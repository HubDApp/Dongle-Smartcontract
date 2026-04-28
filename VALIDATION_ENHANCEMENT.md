# Validation Enhancement for Optional Fields

## Overview
This enhancement implements comprehensive validation for optional fields in the Dongle smart contract, specifically for:
- Project metadata: `website`, `logo_cid`, `metadata_cid`
- Review metadata: `comment_cid`, `ipfs_cid`

## Implementation Summary

### 1. Enhanced URL Validation (`utils.rs`)
**Function:** `is_valid_url(url: &String) -> bool`

**Validation Rules:**
- Length check: Must be between 1 and MAX_WEBSITE_LEN (256) characters
- Protocol check: Must contain `://` protocol separator
- Scheme validation: Must start with `http://` or `https://`
- Minimum domain: Must have at least one character after the scheme

**Example Valid URLs:**
- `https://example.com`
- `http://www.project.io`
- `https://github.com/user/repo`

**Example Invalid URLs:**
- `example.com` (no protocol)
- `ftp://example.com` (unsupported protocol)
- `https://` (no domain)
- `https://` followed by more than 256 characters

### 2. Enhanced IPFS CID Validation (`utils.rs`)
**Function:** `is_valid_ipfs_cid(cid: &String) -> bool`

**Validation Rules:**
- Length check: Must be between 46 and MAX_CID_LEN (128) characters
- CIDv0 validation: 
  - Must start with 'Q'
  - Typically follows "Qm" pattern (base58 encoded)
  - Uses base58 character set
- CIDv1 validation:
  - Alphanumeric lowercase characters (base32 encoding)
  - Pattern: `[a-z0-9]+`

**Example Valid CIDs:**
- CIDv0: `QmXwGvnZ9zLo3vwU5r8G8Q8QvH1qvKrW9d8X8X8X8X8X8X8` (46+ characters starting with Qm)
- CIDv1: `bagaaierahfr...` (lowercase alphanumeric, 46-128 chars)

**Example Invalid CIDs:**
- `QmInvalid123` (too short, not 46+ chars)
- `xyz...` (doesn't start with Q, invalid format)
- `QmX...` (wrong second character, must be 'm')

### 3. New Validation Wrapper Functions (`utils.rs`)

All optional field validation functions:
- Accept `Option<String>` parameter
- Return `Result<(), ContractError>`
- Pass validation if `None` (field not provided)
- Validate using appropriate rules if `Some` (field provided)
- Return `InvalidProjectData` error on validation failure

**New Functions:**
1. `validate_optional_website(website: &Option<String>)` - Uses `is_valid_url`
2. `validate_optional_logo_cid(logo_cid: &Option<String>)` - Uses `is_valid_ipfs_cid`
3. `validate_optional_metadata_cid(metadata_cid: &Option<String>)` - Uses `is_valid_ipfs_cid`
4. `validate_optional_comment_cid(comment_cid: &Option<String>)` - Uses `is_valid_ipfs_cid`

### 4. Integration in Project Registration (`project_registry.rs`)

**File:** `src/project_registry.rs`
**Function:** `register_project()`

**Changes:**
Added validation calls after existing mandatory field validation:
```rust
// Validate optional fields
Utils::validate_optional_website(&params.website)?;
Utils::validate_optional_logo_cid(&params.logo_cid)?;
Utils::validate_optional_metadata_cid(&params.metadata_cid)?;
```

**Location:** Immediately after `category` validation, before owner project count check.

### 5. Integration in Project Updates (`project_registry.rs`)

**File:** `src/project_registry.rs`
**Function:** `update_project()`

**Changes:**
Added validation calls for each optional field when being updated:
```rust
if let Some(value) = params.website {
    Utils::validate_optional_website(&value)?;
    project.website = value;
}
if let Some(value) = params.logo_cid {
    Utils::validate_optional_logo_cid(&value)?;
    project.logo_cid = value;
}
if let Some(value) = params.metadata_cid {
    Utils::validate_optional_metadata_cid(&value)?;
    project.metadata_cid = value;
}
```

**Location:** In the update validation phase, before persisting changes.

### 6. Integration in Review Submission (`review_registry.rs`)

**File:** `src/review_registry.rs`
**Function:** `add_review()`

**Changes:**
Added validation call for `comment_cid`:
```rust
// Validate optional comment CID field
crate::utils::Utils::validate_optional_comment_cid(&comment_cid)?;
```

**Location:** After rating validation, before duplicate review check.

### 7. Integration in Review Updates (`review_registry.rs`)

**File:** `src/review_registry.rs`
**Function:** `update_review()`

**Changes:**
Added validation call for `comment_cid`:
```rust
// Validate optional comment CID field
crate::utils::Utils::validate_optional_comment_cid(&comment_cid)?;
```

**Location:** After rating validation, before retrieving existing review.

## Error Handling

All validation failures return `ContractError::InvalidProjectData` which provides:
- Clear indication that input validation failed
- Consistent error handling across project and review operations
- Proper error propagation using Rust's `?` operator

## Constants Used

From `src/constants.rs`:
- `MAX_WEBSITE_LEN: usize = 256` - Maximum URL length
- `MAX_CID_LEN: usize = 128` - Maximum IPFS CID length

## Backwards Compatibility

The changes are fully backwards compatible:
- Optional fields remain optional (`Option<String>`)
- `None` values bypass validation (no breaking changes)
- Only provided (`Some`) values are validated
- Existing valid projects/reviews continue to work

## Testing Recommendations

### Test Cases for `is_valid_url()`
1. Valid HTTPS URL → true
2. Valid HTTP URL → true
3. URL too long (>256 chars) → false
4. Empty URL → false
5. URL without protocol → false
6. URL with unsupported protocol → false
7. URL with only scheme, no domain → false
8. URLs at boundary (255 chars, 256 chars) → true/false

### Test Cases for `is_valid_ipfs_cid()`
1. Valid CIDv0 (Qm pattern, 46 chars) → true
2. Valid CIDv0 (Qm pattern, 100 chars) → true
3. CID too short (<46 chars) → false
4. CID too long (>128 chars) → false
5. CID with invalid characters → false
6. CID with uppercase (if CIDv1) → false
7. CID without Q prefix (CIDv1 format) → depends on content
8. CIDs at boundary (45 chars, 46 chars, 128 chars, 129 chars) → appropriate

### Test Cases for Optional Field Validation
1. `None` website → passes
2. Valid URL website → passes
3. Invalid URL website → fails
4. `None` logo_cid → passes
5. Valid CID logo_cid → passes
6. Invalid CID logo_cid → fails
7. Similar for metadata_cid and comment_cid

### Integration Test Cases
1. Register project with all optional fields valid → success
2. Register project with invalid website → fails
3. Register project with invalid logo_cid → fails
4. Update project with valid optional fields → success
5. Add review with valid comment_cid → success
6. Add review with invalid comment_cid → fails
7. Update review with valid comment_cid → success
8. Update review with invalid comment_cid → fails

## Files Modified

1. **src/utils.rs**
   - Enhanced `is_valid_ipfs_cid()` implementation
   - Enhanced `is_valid_url()` implementation
   - Added 4 new validation wrapper functions

2. **src/project_registry.rs**
   - Updated `register_project()` - added 3 validation calls
   - Updated `update_project()` - added 3 conditional validation calls

3. **src/review_registry.rs**
   - Updated `add_review()` - added 1 validation call
   - Updated `update_review()` - added 1 validation call

## Line Changes

### utils.rs
- Lines 43-85: Replaced stub `is_valid_ipfs_cid()` with comprehensive validation
- Lines 87-120: Replaced stub `is_valid_url()` with comprehensive validation
- Lines 162-209: Added 4 new optional field validation functions

### project_registry.rs
- Lines 34-36: Added 3 validation calls in `register_project()`
- Lines 174-181: Updated optional field assignments with validation in `update_project()`

### review_registry.rs
- Lines 30-31: Added 1 validation call in `add_review()`
- Lines 129-130: Added 1 validation call in `update_review()`

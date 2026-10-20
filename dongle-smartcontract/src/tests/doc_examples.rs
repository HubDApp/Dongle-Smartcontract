//! Documentation example validity (issue #980).
//!
//! `docs/CONTRACT_INTERFACE.md` is checked against the contract source so docs
//! cannot drift from the real API:
//! - every `### \`fn\`` heading names a public contract function;
//! - every example call `fn(env, ...)` in a ```rust block names a public
//!   contract function and passes the same number of arguments.
//!
//! Representative examples are also executed against a live contract.

use crate::tests::fixtures::{create_test_project, setup_contract};
use soroban_sdk::{testutils::Address as _, Address, Env};
use std::collections::BTreeMap;
use std::string::String as StdString;
use std::vec::Vec as StdVec;

const INTERFACE_DOC: &str = include_str!("../../../docs/CONTRACT_INTERFACE.md");
const LIB_SRC: &str = include_str!("../lib.rs");

/// Split a comma-separated argument list at nesting depth zero.
fn split_args(args: &str) -> StdVec<&str> {
    let (mut depth, mut start, mut out) = (0i32, 0usize, StdVec::new());
    for (i, c) in args.char_indices() {
        match c {
            '(' | '[' | '{' | '<' => depth += 1,
            ')' | ']' | '}' | '>' => depth -= 1,
            ',' if depth == 0 => {
                out.push(&args[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&args[start..]);
    out.into_iter().filter(|a| !a.trim().is_empty()).collect()
}

/// Read the balanced `( ... )` group starting at `open` (index of `(`).
fn balanced(s: &str, open: usize) -> Option<&str> {
    let mut depth = 0i32;
    for (i, c) in s[open..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&s[open + 1..open + i]);
                }
            }
            _ => {}
        }
    }
    None
}

/// Public contract functions and their parameter counts (including `env`).
fn contract_fns() -> BTreeMap<StdString, usize> {
    let mut fns = BTreeMap::new();
    let mut rest = LIB_SRC;
    while let Some(pos) = rest.find("pub fn ") {
        let after = &rest[pos + 7..];
        let name: StdString = after
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if let Some(open) = after.find('(') {
            if let Some(params) = balanced(after, open) {
                fns.entry(name).or_insert(split_args(params).len());
            }
        }
        rest = after;
    }
    fns
}

fn rust_blocks(doc: &str) -> StdVec<&str> {
    doc.split("```rust")
        .skip(1)
        .filter_map(|b| b.split("```").next())
        .collect()
}

#[test]
fn every_documented_function_exists() {
    let fns = contract_fns();
    let missing: StdVec<&str> = INTERFACE_DOC
        .lines()
        .filter_map(|l| l.strip_prefix("### `"))
        .filter_map(|l| l.split('`').next())
        .filter(|name| !fns.contains_key(*name))
        .collect();
    assert!(
        missing.is_empty(),
        "documented but not in contract: {:?}",
        missing
    );
}

#[test]
fn every_example_call_matches_contract_signature() {
    let fns = contract_fns();
    let mut errors = StdVec::new();
    for block in rust_blocks(INTERFACE_DOC) {
        let bytes = block.as_bytes();
        let mut i = 0;
        while let Some(off) = block[i..].find("(env") {
            let open = i + off;
            i = open + 1;
            let start = block[..open]
                .rfind(|c: char| !(c.is_alphanumeric() || c == '_'))
                .map_or(0, |p| p + 1);
            let name = &block[start..open];
            // Skip method calls / paths (e.g. `String::from_slice(&env, ..)`).
            if name.is_empty() || (start > 0 && matches!(bytes[start - 1], b'.' | b':')) {
                continue;
            }
            let Some(&expected) = fns.get(name) else {
                errors.push(std::format!("{name}: not a contract function"));
                continue;
            };
            if let Some(args) = balanced(block, open) {
                let got = split_args(args).len();
                if got != expected {
                    errors.push(std::format!(
                        "{name}: example passes {got} args, expected {expected}"
                    ));
                }
            }
        }
    }
    assert!(errors.is_empty(), "outdated doc examples: {:#?}", errors);
}

#[test]
fn admin_examples_behave_as_documented() {
    // initialize(env, admin_address); add_admin(...); is_admin(...); remove_admin(...)
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let new_admin = Address::generate(&env);

    client.mock_all_auths().add_admin(&admin, &new_admin);
    assert!(client.is_admin(&new_admin));
    client.mock_all_auths().remove_admin(&admin, &new_admin);
    assert!(!client.is_admin(&new_admin));
}

#[test]
fn soft_delete_examples_behave_as_documented() {
    // delete_project(env, project_id, owner_address)?;
    // recover_deleted_project(env, project_id, owner_address)?;
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let id = create_test_project(&client, &owner, "Doc Example");

    client.mock_all_auths().delete_project(&id, &owner);
    assert_eq!(client.get_deleted_projects(&admin).len(), 1);
    client.mock_all_auths().recover_deleted_project(&id, &owner);
    assert_eq!(client.get_deleted_projects(&admin).len(), 0);
    assert_eq!(client.purge_expired_projects(&50), 0);
}
